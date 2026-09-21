use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Determine if an IP address belongs to a private/local network.
pub fn is_private_ip(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback() || (ip.segments()[0] & 0xffc0) == 0xfe80,
    }
}

/// Continuous network quality based on RTT measurement.
#[derive(Debug, Clone, Copy)]
pub struct NetworkQuality {
    pub rtt_ms: f64,
    pub is_private_ip: bool,
}

impl NetworkQuality {
    /// Score in [0.1, 1.0]: higher means better (lower latency) network.
    pub fn score(&self) -> f64 {
        match self.rtt_ms {
            r if r < 2.0   => 1.0,
            r if r < 5.0   => 0.9,
            r if r < 10.0  => 0.8,
            r if r < 20.0  => 0.6,
            r if r < 50.0  => 0.4,
            r if r < 100.0 => 0.2,
            _              => 0.1,
        }
    }

    /// Bootstrap from IP type before any RTT samples are available.
    pub fn from_ip(is_private: bool) -> Self {
        Self {
            rtt_ms: if is_private { 5.0 } else { 50.0 },
            is_private_ip: is_private,
        }
    }

    /// Update RTT with a new EWMA sample.
    pub fn update_rtt(&mut self, rtt_ewma_ms: f64) {
        self.rtt_ms = rtt_ewma_ms;
    }
}

/// Per-frame statistics for BitrateController evaluation.
#[derive(Debug, Clone, Copy)]
pub struct FrameStats {
    pub encode_ms: f64,
    pub frame_bytes: u32,
    pub is_keyframe: bool,
}

/// Decision output from BitrateController.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptiveDecision {
    pub bitrate_bps: u32,
    pub fps: u32,
}

const FPS_TIERS: [f32; 3] = [1.0, 0.5, 0.25];

pub struct BitrateController {
    initial_bitrate: u32,
    current_bitrate: u32,
    target_fps: u32,
    current_fps_tier: usize,
    eval_window: Vec<FrameStats>,
    last_eval_time: Instant,
    eval_interval: Duration,
    network: NetworkQuality,
}

impl BitrateController {
    pub fn new(initial_bitrate: u32, target_fps: u32, network: NetworkQuality) -> Self {
        Self {
            initial_bitrate,
            current_bitrate: initial_bitrate,
            target_fps,
            current_fps_tier: 0,
            eval_window: Vec::with_capacity(128),
            last_eval_time: Instant::now(),
            eval_interval: Duration::from_secs(1),
            network,
        }
    }

    pub fn record_frame(&mut self, stats: FrameStats) { self.eval_window.push(stats); }
    pub fn current_bitrate(&self) -> u32 { self.current_bitrate }
    pub fn current_fps(&self) -> u32 { (self.target_fps as f32 * FPS_TIERS[self.current_fps_tier]) as u32 }
    pub fn target_fps(&self) -> u32 { self.target_fps }
    pub fn network_score(&self) -> f64 { self.network.score() }

    pub fn update_network_rtt(&mut self, rtt_ewma_ms: f64) {
        self.network.update_rtt(rtt_ewma_ms);
    }

    pub fn on_idle_recovery(&mut self) {
        self.eval_window.clear();
        self.current_bitrate = self.initial_bitrate;
        self.current_fps_tier = 0;
        self.last_eval_time = Instant::now();
    }

    pub fn evaluate(&mut self) -> AdaptiveDecision {
        let current_fps = self.current_fps();
        // High-quality network: bypass adaptive logic entirely
        if self.network.score() >= 0.8 {
            self.eval_window.clear();
            self.last_eval_time = Instant::now();
            return AdaptiveDecision { bitrate_bps: self.current_bitrate, fps: current_fps };
        }
        if self.eval_window.is_empty() {
            return AdaptiveDecision { bitrate_bps: self.current_bitrate, fps: current_fps };
        }

        let score = self.network.score();
        let frame_interval_ms = 1000.0 / current_fps as f64;
        let total = self.eval_window.len() as f64;
        let avg_encode_ms = self.eval_window.iter().map(|s| s.encode_ms).sum::<f64>() / total;
        let non_kf: Vec<_> = self.eval_window.iter().filter(|s| !s.is_keyframe).collect();
        let avg_frame_bytes = if non_kf.is_empty() { 0.0 } else {
            non_kf.iter().map(|s| s.frame_bytes as f64).sum::<f64>() / non_kf.len() as f64
        };

        let floor = (self.initial_bitrate as f64 * 0.3) as u32;
        let ceiling = (self.initial_bitrate as f64 * (1.0 + score * 0.5)) as u32;
        let target_frame_bytes = self.current_bitrate as f64 / current_fps as f64 / 8.0;

        // Score-based decrease factor: worse network → more aggressive reduction
        let decrease_factor = if score <= 0.2 { 0.80 } else if score < 0.5 { 0.85 } else { 0.90 };

        let mut new_bitrate = self.current_bitrate;
        let mut new_fps_tier = self.current_fps_tier;

        // Degradation: encode overload takes priority; frame size check only when encode is healthy
        let encode_overloaded = avg_encode_ms > frame_interval_ms * 0.6;
        if encode_overloaded {
            new_bitrate = ((new_bitrate as f64) * decrease_factor) as u32;
        } else if avg_frame_bytes > 0.0 && target_frame_bytes > 0.0 && avg_frame_bytes > target_frame_bytes * 1.5 {
            new_bitrate = ((new_bitrate as f64) * 0.90) as u32;
        }
        new_bitrate = new_bitrate.max(floor).min(ceiling);

        // FPS step down: only if bitrate at floor AND still overloaded
        if new_bitrate <= floor && avg_encode_ms > frame_interval_ms * 0.8 {
            if new_fps_tier < FPS_TIERS.len() - 1 { new_fps_tier += 1; }
        }

        // Recovery (PRIORITY: fps first, then bitrate)
        if avg_encode_ms < frame_interval_ms * 0.3 {
            if new_fps_tier > 0 {
                new_fps_tier -= 1; // fps first
            } else if new_bitrate < ceiling {
                new_bitrate = (((new_bitrate as f64) * 1.10) as u32).min(ceiling);
            }
        }

        self.current_bitrate = new_bitrate;
        self.current_fps_tier = new_fps_tier;
        self.eval_window.clear();
        self.last_eval_time = Instant::now();
        AdaptiveDecision { bitrate_bps: new_bitrate, fps: self.current_fps() }
    }

    pub fn should_evaluate(&self) -> bool {
        self.last_eval_time.elapsed() >= self.eval_interval && !self.eval_window.is_empty()
    }


}

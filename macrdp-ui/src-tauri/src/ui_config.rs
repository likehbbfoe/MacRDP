//! Independent UI configuration with persistent storage at ~/.macrdp/config-ui.toml

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// UI-specific configuration. All fields have sensible defaults so
/// `UiConfig::default()` is always usable.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    // ── Server ──────────────────────────────────────────────
    pub port: u16,
    pub frame_rate: u32,
    pub bitrate_mbps: u32,
    pub encoder: String,
    pub chroma_mode: String,
    pub bind_address: String,
    pub max_connections: u32,
    pub idle_timeout_secs: u64,

    // ── Auth ────────────────────────────────────────────────
    pub username: String,
    pub password: String,

    // ── Display ─────────────────────────────────────────────
    #[serde(alias = "hidpi_scale")]
    pub resolution: String,
    pub show_cursor: bool,

    // ── Application ─────────────────────────────────────────
    pub log_level: String,
    pub theme: String,
    pub autostart: bool,
}

impl std::fmt::Debug for UiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiConfig")
            .field("port", &self.port)
            .field("frame_rate", &self.frame_rate)
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            port: 3389,
            frame_rate: 60,
            bitrate_mbps: 50,
            encoder: "auto".to_string(),
            chroma_mode: "avc420".to_string(),
            bind_address: "0.0.0.0".to_string(),
            max_connections: 3,
            idle_timeout_secs: 1800,
            username: "macrdp".to_string(),
            password: String::new(),
            resolution: "auto".to_string(),
            show_cursor: false,
            log_level: "warn".to_string(),
            theme: "system".to_string(),
            autostart: false,
        }
    }
}

/// Returns the config file path: `~/.macrdp/config-ui.toml`
pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".macrdp")
        .join("config-ui.toml")
}

impl UiConfig {
    /// Load from `~/.macrdp/config-ui.toml`, creating defaults if the file
    /// does not exist or is unparseable.
    pub fn load() -> Result<Self, String> {
        let path = config_path();
        if !path.exists() {
            let cfg = Self::default();
            // Try to persist the defaults so the user has a file to edit.
            let _ = cfg.save();
            return Ok(cfg);
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("parse config: {e}"))
    }

    /// Serialize to TOML and write to `~/.macrdp/config-ui.toml`.
    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create config dir: {e}"))?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("serialize config: {e}"))?;
        std::fs::write(&path, content).map_err(|e| format!("write config: {e}"))
    }

    /// Update a single field by key name from a JSON value.
    /// Returns whether capture, encoder or logging changes require a service restart.
    /// Invalid updates leave the current configuration unchanged.
    pub fn set_field(
        &mut self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<bool, String> {
        let mut candidate = self.clone();
        let restart_required = candidate.apply_field(key, value)?;
        candidate.to_server_config().validate().map_err(|err| err.to_string())?;
        *self = candidate;
        Ok(restart_required)
    }

    fn apply_field(&mut self, key: &str, value: &serde_json::Value) -> Result<bool, String> {
        match key {
            "port" => {
                self.port = u16::try_from(value.as_u64().ok_or("port must be a number")?)
                    .map_err(|_| "port is out of range")?;
            }
            "frame_rate" => {
                self.frame_rate = u32::try_from(value.as_u64().ok_or("frame_rate must be a number")?)
                    .map_err(|_| "frame_rate is out of range")?;
            }
            "bitrate_mbps" => {
                self.bitrate_mbps = u32::try_from(value.as_u64().ok_or("bitrate_mbps must be a number")?)
                    .map_err(|_| "bitrate_mbps is out of range")?;
            }
            "encoder" => {
                self.encoder = macrdp_core::EncoderPreference::try_from_str_opt(Some(
                    value.as_str().ok_or("encoder must be a string")?
                )).map_err(|err| err.to_string())?.as_str().to_owned();
            }
            "chroma_mode" => {
                self.chroma_mode = value
                    .as_str()
                    .ok_or("chroma_mode must be a string")?
                    .to_string();
            }
            "bind_address" => {
                self.bind_address = value
                    .as_str()
                    .ok_or("bind_address must be a string")?
                    .to_string();
            }
            "max_connections" => {
                self.max_connections = u32::try_from(value.as_u64().ok_or("max_connections must be a number")?)
                    .map_err(|_| "max_connections is out of range")?;
            }
            "idle_timeout_secs" => {
                self.idle_timeout_secs = value
                    .as_u64()
                    .ok_or("idle_timeout_secs must be a number")?;
            }
            "username" => {
                self.username = value
                    .as_str()
                    .ok_or("username must be a string")?
                    .to_string();
            }
            "password" => {
                self.password = value
                    .as_str()
                    .ok_or("password must be a string")?
                    .to_string();
            }
            "resolution" | "hidpi_scale" => {
                self.resolution = value
                    .as_str()
                    .or_else(|| value.as_u64().map(|_| ""))
                    .ok_or("resolution must be a string")?
                    .to_string();
                if self.resolution.is_empty() {
                    self.resolution = value.as_u64().unwrap().to_string();
                }
            }
            "show_cursor" => {
                self.show_cursor = value
                    .as_bool()
                    .ok_or("show_cursor must be a boolean")?;
            }
            "log_level" => {
                self.log_level = value
                    .as_str()
                    .ok_or("log_level must be a string")?
                    .to_string();
            }
            "theme" => {
                self.theme = value
                    .as_str()
                    .ok_or("theme must be a string")?
                    .to_string();
            }
            "autostart" => {
                self.autostart = value
                    .as_bool()
                    .ok_or("autostart must be a boolean")?;
            }
            _ => return Err(format!("unknown config key: {key}")),
        }

        // Capture and encoder changes must take effect together in a new session.
        let restart_required = matches!(key,
            "frame_rate" | "encoder" | "chroma_mode" | "resolution" |
            "hidpi_scale" | "show_cursor" | "log_level"
        );
        Ok(restart_required)
    }

    /// Convert to the core library's `ServerConfig` used to start the server.
    pub fn to_server_config(&self) -> macrdp_core::ServerConfig {
        macrdp_core::ServerConfig {
            port: self.port,
            frame_rate: self.frame_rate,
            width: 0,  // auto-detect
            height: 0, // auto-detect
            username: Some(self.username.clone()),
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
            cert_path: None,
            key_path: None,
            idle_timeout_secs: self.idle_timeout_secs,
            log_level: Some(self.log_level.clone()),
            quality: None,
            encoder: Some(self.encoder.clone()),
            chroma_mode: Some(self.chroma_mode.clone()),
            resolution: Some(self.resolution.clone()),
            show_cursor: Some(self.show_cursor),
            bitrate_mbps: Some(self.bitrate_mbps),
            audio: macrdp_core::AudioConfig::default(),
            clipboard: macrdp_core::ClipboardConfig::default(),
        }
    }
}

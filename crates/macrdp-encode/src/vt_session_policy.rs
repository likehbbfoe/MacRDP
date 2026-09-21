//! Selection policy for hardware-only VideoToolbox sessions.

use anyhow::{anyhow, Result};

pub(super) fn try_hardware_modes<T>(mut create: impl FnMut(bool) -> Result<T>) -> Result<T> {
    match create(true) {
        Ok(session) => Ok(session),
        Err(low_latency_error) => {
            tracing::debug!(error = %low_latency_error, "Low-latency hardware unavailable; trying ordinary hardware");
            create(false).map_err(|error| anyhow!(
                "No compatible hardware session: low-latency: {low_latency_error:#}; ordinary: {error:#}"
            ))
        }
    }
}

pub(super) fn try_profiles<T>(mut create: impl FnMut(&'static str) -> Result<T>) -> Result<T> {
    let mut failures = Vec::new();
    for profile in [
        "H264_ConstrainedBaseline_AutoLevel",
        "H264_Baseline_AutoLevel",
        "H264_Main_AutoLevel",
        "H264_High_AutoLevel",
    ] {
        match create(profile) {
            Ok(session) => return Ok(session),
            Err(error) => failures.push(format!("{profile}: {error:#}")),
        }
    }
    Err(anyhow!("No supported H.264 profile: {}", failures.join("; ")))
}

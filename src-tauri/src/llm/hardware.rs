use std::process::Command;

use crate::llm::types::HardwareInfo;
use crate::utils::error::{AppError, AppResult};

pub fn detect_hardware() -> HardwareInfo {
    if let Some(gpu) = detect_nvidia_gpu() {
        return gpu;
    }

    HardwareInfo {
        gpu_available: false,
        gpu_name: None,
        vram_total_mb: None,
        vram_free_mb: None,
        recommended_gpu_layers: 0,
        backend: "cpu".to_string(),
    }
}

fn detect_nvidia_gpu() -> Option<HardwareInfo> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }

    let name = parts[0].to_string();
    let vram_total_mb = parts[1].parse::<u64>().ok();
    let vram_free_mb = parts[2].parse::<u64>().ok();

    let recommended_gpu_layers = vram_total_mb
        .map(|mb| {
            if mb >= 12_000 {
                99
            } else if mb >= 8_000 {
                35
            } else if mb >= 6_000 {
                20
            } else {
                8
            }
        })
        .unwrap_or(0);

    Some(HardwareInfo {
        gpu_available: true,
        gpu_name: Some(name),
        vram_total_mb,
        vram_free_mb,
        recommended_gpu_layers,
        backend: "cuda".to_string(),
    })
}

pub fn gpu_layers_flag(hardware: &HardwareInfo) -> Vec<String> {
    if hardware.gpu_available && hardware.recommended_gpu_layers > 0 {
        vec![
            "-ngl".to_string(),
            hardware.recommended_gpu_layers.to_string(),
        ]
    } else {
        vec!["-ngl".to_string(), "0".to_string()]
    }
}

pub fn require_nvidia_smi() -> AppResult<()> {
    let status = Command::new("nvidia-smi").arg("--version").status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err(AppError::Other(
            "nvidia-smi not available on PATH".to_string(),
        )),
    }
}

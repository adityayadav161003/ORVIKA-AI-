use std::process::Command;

use crate::llm::types::HardwareInfo;
use crate::utils::error::{AppError, AppResult};

fn detect_cpu_info() -> (String, u32, u32, f64) {
    if cfg!(target_os = "windows") {
        let name_out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Name",
            ])
            .output();
        let cpu_brand = name_out
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let cores_out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty NumberOfCores",
            ])
            .output();
        let physical_cores = cores_out
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<u32>()
                    .ok()
            })
            .unwrap_or(4);

        let logical_out = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty NumberOfLogicalProcessors"])
            .output();
        let logical_cores = logical_out
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<u32>()
                    .ok()
            })
            .unwrap_or(8);

        let mem_out = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-CimInstance Win32_ComputerSystem | Select-Object -ExpandProperty TotalPhysicalMemory"])
            .output();
        let total_memory_gb = mem_out
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
            .map(|bytes| bytes / (1024.0 * 1024.0 * 1024.0))
            .unwrap_or(16.0);

        (cpu_brand, physical_cores, logical_cores, total_memory_gb)
    } else {
        ("Unknown CPU".to_string(), 4, 8, 16.0)
    }
}

pub fn detect_hardware() -> HardwareInfo {
    let (cpu_brand, physical_cores, logical_cores, total_memory_gb) = detect_cpu_info();

    let mut gpu = detect_nvidia_gpu();
    if let Some(ref mut info) = gpu {
        info.cpu_brand = cpu_brand;
        info.physical_cores = physical_cores;
        info.logical_cores = logical_cores;
        info.total_memory_gb = total_memory_gb;
        info.has_nvidia_gpu = true;
        return info.clone();
    }

    HardwareInfo {
        gpu_available: false,
        gpu_name: None,
        vram_total_mb: None,
        vram_free_mb: None,
        recommended_gpu_layers: 0,
        backend: "cpu".to_string(),
        cpu_brand,
        physical_cores,
        logical_cores,
        total_memory_gb,
        has_nvidia_gpu: false,
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
        cpu_brand: "Unknown".to_string(),
        physical_cores: 0,
        logical_cores: 0,
        total_memory_gb: 0.0,
        has_nvidia_gpu: true,
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

use std::path::{Path, PathBuf};
use std::process::Command;
use crate::utils::error::{AppError, AppResult};

/// Spawns an `ffmpeg` process to extract raw PCM audio (16kHz, mono, 16-bit)
/// from a video/audio file to a target WAV file path.
pub fn extract_audio(input_path: &Path, output_path: &Path) -> AppResult<()> {
    // Check if input exists
    if !input_path.exists() {
        return Err(AppError::Other(format!(
            "Input file does not exist: {:?}",
            input_path
        )));
    }

    // Verify ffmpeg is available
    let test_cmd = Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if test_cmd.is_err() || !test_cmd.unwrap().success() {
        return Err(AppError::Other(
            "FFmpeg executable not found in system PATH. Please install FFmpeg to process media files.".into()
        ));
    }

    // Execute ffmpeg extraction:
    // -y: overwrite output
    // -vn: disable video recording
    // -acodec pcm_s16le: 16-bit PCM codec
    // -ar 16000: 16kHz sample rate
    // -ac 1: mono channel
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input_path)
        .arg("-vn")
        .arg("-acodec")
        .arg("pcm_s16le")
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        .arg(output_path)
        .status()
        .map_err(|e| AppError::Other(format!("Failed to spawn ffmpeg: {}", e)))?;

    if !status.success() {
        return Err(AppError::Other(format!(
            "FFmpeg extraction failed with exit code: {:?}",
            status.code()
        )));
    }

    Ok(())
}

use std::path::{Path, PathBuf};
use std::fs;
use crate::python::manager::PythonManager;
use crate::media::types::MediaTranscript;
use crate::media::extractor;
use crate::utils::error::{AppError, AppResult};

/// Standardizes a video or audio file into a temporary WAV format,
/// executes Whisper transcription via the Python virtual environment,
/// cleans up the temporary file, and returns the transcription segments.
pub fn transcribe(
    media_path: &Path,
    python_manager: &PythonManager,
    model_size: &str,
    app_data_dir: &Path,
) -> AppResult<MediaTranscript> {
    // Ensure temp directory exists
    let temp_dir = app_data_dir.join("temp");
    fs::create_dir_all(&temp_dir).map_err(|e| AppError::Other(e.to_string()))?;
    
    // Create a random temporary WAV filename
    let temp_wav_path = temp_dir.join(format!(
        "transcode_{}.wav",
        uuid::Uuid::new_v4()
    ));

    tracing::info!("Converting media file to 16kHz WAV: {:?}", temp_wav_path);
    
    // Convert video or audio file to PCM WAV
    if let Err(err) = extractor::extract_audio(media_path, &temp_wav_path) {
        let _ = fs::remove_file(&temp_wav_path);
        return Err(err);
    }

    // Call python transcriber
    tracing::info!("Starting Python transcription for standard WAV...");
    let result = python_manager.transcribe_audio(&temp_wav_path, model_size);

    // Cleanup temp WAV file
    tracing::info!("Cleaning up temporary WAV file...");
    let _ = fs::remove_file(&temp_wav_path);

    result
}

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{Write, BufReader, BufRead};

use serde::{Deserialize, Serialize};

use crate::utils::error::{AppError, AppResult};

#[derive(Serialize)]
struct ParseRequest {
    file_path: String,
}

#[derive(Deserialize)]
struct ParseResponse {
    status: String,
    content: Option<String>,
    error: Option<String>,
    traceback: Option<String>,
}

pub struct PythonManager {
    app_data_dir: PathBuf,
}

impl PythonManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    /// Returns the path to the venv Python executable, initialising the venv
    /// and installing requirements if needed.  This is the same as the private
    /// `ensure_venv()` but exposed publicly so callers outside this module can
    /// obtain the executable path without duplicating logic.
    pub fn python_exe(&self) -> AppResult<PathBuf> {
        self.ensure_venv()
    }

    /// Resolves a Python script name (e.g. `"parser.py"`) to an absolute path,
    /// searching in the standard development locations first, then writing the
    /// embedded script to `app_data_dir` for production/installed builds.
    ///
    /// The `script_name` must match the filename embedded via `include_str!` in
    /// the per-script fallback arms below.  Returns an error if the script
    /// cannot be found or written.
    pub fn resolve_script(&self, script_name: &str) -> AppResult<PathBuf> {
        // Try dev paths first
        let candidates = [
            std::env::current_dir()
                .unwrap_or_default()
                .join("python")
                .join(script_name),
            std::env::current_dir()
                .unwrap_or_default()
                .join("src-tauri")
                .join("python")
                .join(script_name),
        ];
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        // Production fallback — write embedded bytes to app_data_dir
        let target = self.app_data_dir.join(script_name);
        if target.exists() {
            return Ok(target);
        }

        let content = match script_name {
            "parser.py"     => include_str!("../../python/parser.py"),
            "embedder.py"   => include_str!("../../python/embedder.py"),
            "ocr_parser.py" => include_str!("../../python/ocr_parser.py"),
            "transcriber.py"=> include_str!("../../python/transcriber.py"),
            other => {
                return Err(AppError::Other(format!(
                    "Unknown Python script '{}' — add it to resolve_script()",
                    other
                )))
            }
        };

        std::fs::write(&target, content).map_err(|e| {
            AppError::Other(format!("Failed to write {} to app_data_dir: {}", script_name, e))
        })?;

        Ok(target)
    }


    fn ensure_venv(&self) -> AppResult<PathBuf> {
        let venv_path = self.app_data_dir.join("python_venv");
        let python_exe = if cfg!(windows) {
            venv_path.join("Scripts").join("python.exe")
        } else {
            venv_path.join("bin").join("python")
        };

        if !python_exe.exists() {
            tracing::info!("Creating Python virtual environment...");
            
            // Assuming `python` or `python3` is available globally
            let sys_python = if cfg!(windows) { "python" } else { "python3" };
            
            let status = Command::new(sys_python)
                .args(["-m", "venv", venv_path.to_str().unwrap()])
                .status()
                .map_err(|e| AppError::Other(format!("Failed to run python -m venv: {}", e)))?;

            if !status.success() {
                return Err(AppError::Other("Failed to create virtual environment".into()));
            }

            tracing::info!("Installing Python requirements...");
            
            let pip_exe = if cfg!(windows) {
                venv_path.join("Scripts").join("pip.exe")
            } else {
                venv_path.join("bin").join("pip")
            };

            let mut req_path = std::env::current_dir()
                .unwrap_or_default()
                .join("python")
                .join("requirements.txt");

            if !req_path.exists() {
                req_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("src-tauri")
                    .join("python")
                    .join("requirements.txt");
            }

            if !req_path.exists() {
                let target_req = self.app_data_dir.join("requirements.txt");
                let req_content = include_str!("../../python/requirements.txt");
                if let Err(e) = std::fs::write(&target_req, req_content) {
                    tracing::error!("Failed to write fallback requirements.txt: {}", e);
                } else {
                    req_path = target_req;
                }
            }

            let mut pip_cmd = Command::new(&pip_exe);
            pip_cmd.arg("install");
            
            if req_path.exists() {
                pip_cmd.arg("-r").arg(&req_path);
            } else {
                pip_cmd.arg("markitdown")
                    .arg("torch")
                    .arg("sentence-transformers")
                    .arg("faster-whisper")
                    .arg("pytesseract")
                    .arg("pdf2image")
                    .arg("Pillow");
            }

            let pip_status = pip_cmd
                .status()
                .map_err(|e| AppError::Other(format!("Failed to run pip: {}", e)))?;

            if !pip_status.success() {
                return Err(AppError::Other("Failed to install Python dependencies".into()));
            }
        }

        Ok(python_exe)
    }

    pub fn parse_document(&self, file_path: &Path) -> AppResult<String> {
        let python_exe = self.ensure_venv()?;

        let mut script_path = std::env::current_dir()
            .unwrap_or_default()
            .join("python")
            .join("parser.py");

        if !script_path.exists() {
            script_path = std::env::current_dir()
                .unwrap_or_default()
                .join("src-tauri")
                .join("python")
                .join("parser.py");
        }

        // Note: For a production build, the script should be written from `include_str!` to a temp dir.
        // But for development, we will use the local path.
        let actual_script_path = if script_path.exists() {
            script_path
        } else {
            // Write script to app_data_dir for production
            let target_path = self.app_data_dir.join("parser.py");
            if !target_path.exists() {
                let script_content = include_str!("../../python/parser.py");
                std::fs::write(&target_path, script_content)
                    .map_err(|e| AppError::Other(format!("Failed to write parser.py: {}", e)))?;
            }
            target_path
        };

        let mut child = Command::new(&python_exe)
            .arg(&actual_script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to spawn Python parser: {}", e)))?;

        let req = ParseRequest {
            file_path: file_path.to_string_lossy().to_string(),
        };
        
        let json_line = serde_json::to_string(&req).unwrap() + "\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json_line.as_bytes())
                .map_err(|e| AppError::Other(format!("Failed to write to python stdin: {}", e)))?;
        }

        let mut stdout = child.stdout.take().expect("Failed to open stdout");
        let mut reader = BufReader::new(&mut stdout);
        
        let mut response_line = String::new();
        reader.read_line(&mut response_line)
            .map_err(|e| AppError::Other(format!("Failed to read python stdout: {}", e)))?;

        let _ = child.wait();

        if response_line.is_empty() {
            return Err(AppError::Other("Python parser returned no output".into()));
        }

        let response: ParseResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Other(format!("Failed to parse Python response: {}", e)))?;

        if response.status == "success" {
            Ok(response.content.unwrap_or_default())
        } else {
            Err(AppError::Other(format!(
                "Parse error: {} {:?}",
                response.error.unwrap_or_default(),
                response.traceback
            )))
        }
    }
    pub fn embed_chunks(&self, chunks: Vec<String>) -> AppResult<Vec<Vec<f32>>> {
        let python_exe = self.ensure_venv()?;

        let mut script_path = std::env::current_dir()
            .unwrap_or_default()
            .join("python")
            .join("embedder.py");

        if !script_path.exists() {
            script_path = std::env::current_dir()
                .unwrap_or_default()
                .join("src-tauri")
                .join("python")
                .join("embedder.py");
        }

        let actual_script_path = if script_path.exists() {
            script_path
        } else {
            let target_path = self.app_data_dir.join("embedder.py");
            if !target_path.exists() {
                let script_content = include_str!("../../python/embedder.py");
                std::fs::write(&target_path, script_content)
                    .map_err(|e| AppError::Other(format!("Failed to write embedder.py: {}", e)))?;
            }
            target_path
        };

        let mut child = Command::new(&python_exe)
            .arg(&actual_script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to spawn Python embedder: {}", e)))?;

        #[derive(Serialize)]
        struct EmbedRequest {
            chunks: Vec<String>,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            status: String,
            embeddings: Option<Vec<Vec<f32>>>,
            error: Option<String>,
            traceback: Option<String>,
        }

        let req = EmbedRequest { chunks };
        let json_line = serde_json::to_string(&req).unwrap() + "\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json_line.as_bytes())
                .map_err(|e| AppError::Other(format!("Failed to write to python stdin: {}", e)))?;
        }

        let mut stdout = child.stdout.take().expect("Failed to open stdout");
        let mut reader = BufReader::new(&mut stdout);
        
        let mut response_line = String::new();
        reader.read_line(&mut response_line)
            .map_err(|e| AppError::Other(format!("Failed to read python stdout: {}", e)))?;

        let _ = child.wait();

        if response_line.is_empty() {
            return Err(AppError::Other("Python embedder returned no output".into()));
        }

        let response: EmbedResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Other(format!("Failed to parse Python response: {}", e)))?;

        if response.status == "success" {
            Ok(response.embeddings.unwrap_or_default())
        } else {
            Err(AppError::Other(format!(
                "Embed error: {} {:?}",
                response.error.unwrap_or_default(),
                response.traceback
            )))
        }
    }

    pub fn transcribe_audio(&self, file_path: &Path, model_size: &str) -> AppResult<crate::media::types::MediaTranscript> {
        let python_exe = self.ensure_venv()?;

        let mut script_path = std::env::current_dir()
            .unwrap_or_default()
            .join("python")
            .join("transcriber.py");

        if !script_path.exists() {
            script_path = std::env::current_dir()
                .unwrap_or_default()
                .join("src-tauri")
                .join("python")
                .join("transcriber.py");
        }

        let actual_script_path = if script_path.exists() {
            script_path
        } else {
            let target_path = self.app_data_dir.join("transcriber.py");
            if !target_path.exists() {
                let script_content = include_str!("../../python/transcriber.py");
                std::fs::write(&target_path, script_content)
                    .map_err(|e| AppError::Other(format!("Failed to write transcriber.py: {}", e)))?;
            }
            target_path
        };

        let mut child = Command::new(&python_exe)
            .arg(&actual_script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to spawn Python transcriber: {}", e)))?;

        #[derive(Serialize)]
        struct TranscribeRequest {
            file_path: String,
            model_size: String,
        }

        #[derive(Deserialize)]
        struct TranscribeResponse {
            status: String,
            segments: Option<Vec<crate::media::types::TranscriptSegment>>,
            language: Option<String>,
            duration: Option<f64>,
            error: Option<String>,
            traceback: Option<String>,
        }

        let req = TranscribeRequest {
            file_path: file_path.to_string_lossy().to_string(),
            model_size: model_size.to_string(),
        };
        let json_line = serde_json::to_string(&req).unwrap() + "\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json_line.as_bytes())
                .map_err(|e| AppError::Other(format!("Failed to write to python stdin: {}", e)))?;
        }

        let mut stdout = child.stdout.take().expect("Failed to open stdout");
        let mut reader = BufReader::new(&mut stdout);
        
        let mut response_line = String::new();
        reader.read_line(&mut response_line)
            .map_err(|e| AppError::Other(format!("Failed to read python stdout: {}", e)))?;

        let _ = child.wait();

        if response_line.is_empty() {
            return Err(AppError::Other("Python transcriber returned no output".into()));
        }

        let response: TranscribeResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Other(format!("Failed to parse Python response: {}", e)))?;

        if response.status == "success" {
            Ok(crate::media::types::MediaTranscript {
                segments: response.segments.unwrap_or_default(),
                language: response.language.unwrap_or_else(|| "en".to_string()),
                duration: response.duration.unwrap_or(0.0),
            })
        } else {
            Err(AppError::Other(format!(
                "Transcription error: {} {:?}",
                response.error.unwrap_or_default(),
                response.traceback
            )))
        }
    }

    pub fn perform_ocr(&self, file_path: &Path) -> AppResult<(String, Vec<f32>)> {
        let python_exe = self.ensure_venv()?;

        let mut script_path = std::env::current_dir()
            .unwrap_or_default()
            .join("python")
            .join("ocr_parser.py");

        if !script_path.exists() {
            script_path = std::env::current_dir()
                .unwrap_or_default()
                .join("src-tauri")
                .join("python")
                .join("ocr_parser.py");
        }

        let actual_script_path = if script_path.exists() {
            script_path
        } else {
            let target_path = self.app_data_dir.join("ocr_parser.py");
            if !target_path.exists() {
                let script_content = include_str!("../../python/ocr_parser.py");
                std::fs::write(&target_path, script_content)
                    .map_err(|e| AppError::Other(format!("Failed to write ocr_parser.py: {}", e)))?;
            }
            target_path
        };

        let mut child = Command::new(&python_exe)
            .arg(&actual_script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to spawn Python OCR parser: {}", e)))?;

        #[derive(Serialize)]
        struct OcrRequest {
            file_path: String,
        }

        #[derive(Deserialize)]
        struct OcrResponse {
            status: String,
            content: Option<String>,
            confidence_per_page: Option<Vec<f32>>,
            error: Option<String>,
            traceback: Option<String>,
        }

        let req = OcrRequest {
            file_path: file_path.to_string_lossy().to_string(),
        };
        let json_line = serde_json::to_string(&req).unwrap() + "\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json_line.as_bytes())
                .map_err(|e| AppError::Other(format!("Failed to write to python stdin: {}", e)))?;
        }

        let mut stdout = child.stdout.take().expect("Failed to open stdout");
        let mut reader = BufReader::new(&mut stdout);
        
        let mut response_line = String::new();
        reader.read_line(&mut response_line)
            .map_err(|e| AppError::Other(format!("Failed to read python stdout: {}", e)))?;

        let _ = child.wait();

        if response_line.is_empty() {
            return Err(AppError::Other("Python OCR parser returned no output".into()));
        }

        let response: OcrResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Other(format!("Failed to parse Python OCR response: {}", e)))?;

        if response.status == "success" {
            Ok((
                response.content.unwrap_or_default(),
                response.confidence_per_page.unwrap_or_default()
            ))
        } else {
            Err(AppError::Other(format!(
                "OCR error: {} {:?}",
                response.error.unwrap_or_default(),
                response.traceback
            )))
        }
    }
}


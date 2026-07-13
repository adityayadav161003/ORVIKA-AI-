use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::Client;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::Database;
use crate::llm::config::{self, server_base_url, DEFAULT_HOST, DEFAULT_PORT, HEALTH_PATH};
use crate::llm::hardware::{detect_hardware, gpu_layers_flag};
use crate::llm::types::{LlmServerState, LlmStatus};
use crate::utils::error::{AppError, AppResult};

pub struct LlmRuntime {
    pub app_data_dir: std::path::PathBuf,
    state: Mutex<RuntimeInner>,
    cancel_health: AtomicBool,
    pub cancel_inference: AtomicBool,
    client: Client,
    blocking_client: reqwest::blocking::Client,
}

struct RuntimeInner {
    state: LlmServerState,
    child: Option<Child>,
    host: String,
    port: u16,
    model_path: Option<String>,
    last_error: Option<String>,
}

impl LlmRuntime {
    pub fn app_data_dir(&self) -> std::path::PathBuf {
        self.app_data_dir.clone()
    }

    pub fn new(app_data_dir: std::path::PathBuf) -> Self {
        std::fs::create_dir_all(config::binaries_dir(&app_data_dir)).ok();
        std::fs::create_dir_all(config::models_dir(&app_data_dir)).ok();

        Self {
            app_data_dir,
            state: Mutex::new(RuntimeInner {
                state: LlmServerState::Stopped,
                child: None,
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                model_path: None,
                last_error: None,
            }),
            cancel_health: AtomicBool::new(false),
            cancel_inference: AtomicBool::new(false),
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("reqwest client"),
            blocking_client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("reqwest blocking client"),
        }
    }

    pub fn status(&self) -> LlmStatus {
        let inner = self.state.lock().expect("runtime lock");
        let binary_path = config::resolve_llama_server_binary(&self.app_data_dir)
            .map(|p| p.display().to_string());

        let healthy =
            matches!(inner.state, LlmServerState::Running) && self.health_check_sync(inner.port);

        LlmStatus {
            state: inner.state,
            host: inner.host.clone(),
            port: inner.port,
            pid: inner.child.as_ref().map(|c| c.id()),
            healthy,
            model_path: inner.model_path.clone(),
            last_error: inner.last_error.clone(),
            binary_path,
        }
    }

    pub fn set_model_path(&self, path: Option<String>) {
        let mut inner = self.state.lock().expect("runtime lock");
        inner.model_path = path;
    }

    pub fn start(self: &Arc<Self>, app: &AppHandle) -> AppResult<LlmStatus> {
        let binary = config::resolve_llama_server_binary(&self.app_data_dir).ok_or_else(|| {
            AppError::Config(
                "llama-server not found. Set LLAMA_SERVER_PATH or place binary in app data bin/"
                    .into(),
            )
        })?;

        let model_path = {
            let inner = self.state.lock().expect("runtime lock");
            inner.model_path.clone()
        };

        let model_path = model_path.ok_or_else(|| {
            AppError::Config("No model selected. Download and activate a model first.".into())
        })?;

        if !Path::new(&model_path).is_file() {
            return Err(AppError::Config(format!(
                "Model file not found: {model_path}"
            )));
        }

        self.stop()?;

        {
            let mut inner = self.state.lock().expect("runtime lock");
            inner.state = LlmServerState::Starting;
            inner.last_error = None;
            self.emit_status(app, &inner);
        }

        let hardware = detect_hardware();
        let mut args = vec![
            "-m".to_string(),
            model_path.clone(),
            "--host".to_string(),
            DEFAULT_HOST.to_string(),
            "--port".to_string(),
            DEFAULT_PORT.to_string(),
        ];
        args.extend(gpu_layers_flag(&hardware));

        // Expose performance parameters from settings database
        let context_size = if let Some(db) = app.try_state::<Arc<Database>>() {
            db.with_connection(|conn| crate::db::settings_repo::get(conn, "gpu_context_size"))
                .ok()
                .flatten()
                .unwrap_or_else(|| "2048".to_string())
        } else {
            "2048".to_string()
        };

        let batch_size = if let Some(db) = app.try_state::<Arc<Database>>() {
            db.with_connection(|conn| crate::db::settings_repo::get(conn, "gpu_batch_size"))
                .ok()
                .flatten()
                .unwrap_or_else(|| "512".to_string())
        } else {
            "512".to_string()
        };

        args.push("-c".to_string());
        args.push(context_size);
        args.push("-b".to_string());
        args.push(batch_size);

        tracing::info!(?binary, ?args, "Starting llama-server");

        let child = Command::new(&binary)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| AppError::Other(format!("Failed to start llama-server: {err}")))?;

        let pid = child.id();

        {
            let mut inner = self.state.lock().expect("runtime lock");
            inner.child = Some(child);
            inner.state = LlmServerState::Running;
            inner.model_path = Some(model_path);
            self.emit_status(app, &inner);
        }

        self.wait_until_healthy(DEFAULT_PORT, 60)?;

        self.spawn_health_monitor(app.clone(), pid);

        Ok(self.status())
    }

    pub fn restart(self: &Arc<Self>, app: &AppHandle) -> AppResult<LlmStatus> {
        self.stop()?;
        self.cancel_health.store(false, Ordering::SeqCst);
        self.start(app)
    }

    pub fn stop(&self) -> AppResult<()> {
        self.cancel_health.store(true, Ordering::SeqCst);

        let mut inner = self.state.lock().expect("runtime lock");
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        inner.state = LlmServerState::Stopped;
        inner.last_error = None;
        Ok(())
    }

    pub fn base_url(&self) -> String {
        let inner = self.state.lock().expect("runtime lock");
        server_base_url(&inner.host, inner.port)
    }

    pub fn http_client(&self) -> &Client {
        &self.client
    }

    pub fn cancel_inference(&self) {
        self.cancel_inference.store(true, Ordering::SeqCst);
    }

    pub fn ensure_running(&self) -> AppResult<()> {
        let status = self.status();
        if status.state == LlmServerState::Running && status.healthy {
            return Ok(());
        }
        Err(AppError::Other(
            "LLM server is not running. Start the server from the LLM panel.".into(),
        ))
    }

    fn wait_until_healthy(&self, port: u16, attempts: u32) -> AppResult<()> {
        for _ in 0..attempts {
            if self.health_check_sync(port) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        self.mark_crashed("Server did not become healthy within timeout");
        Err(AppError::Other(
            "llama-server failed health check after start".into(),
        ))
    }

    fn health_check_sync(&self, port: u16) -> bool {
        let url = format!("{}{}", server_base_url(DEFAULT_HOST, port), HEALTH_PATH);
        self.blocking_client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub fn spawn_health_monitor(self: &Arc<Self>, app: AppHandle, pid: u32) {
        self.cancel_health.store(false, Ordering::SeqCst);
        let blocking_client = self.blocking_client.clone();
        let port = DEFAULT_PORT;
        let runtime = Arc::clone(self);

        std::thread::spawn(move || {
            while !runtime.cancel_health.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(3));

                let running = {
                    let inner = runtime.state.lock().expect("runtime lock");
                    inner.child.as_ref().map(|c| c.id() == pid).unwrap_or(false)
                };

                if !running {
                    runtime.mark_crashed("Process exited unexpectedly");
                    let _ = app.emit("llm-runtime-status", runtime.status());
                    break;
                }

                let url = format!("{}{}", server_base_url(DEFAULT_HOST, port), HEALTH_PATH);
                let healthy = blocking_client
                    .get(&url)
                    .send()
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);

                if !healthy {
                    runtime.mark_crashed("Health check failed");
                    let _ = app.emit("llm-runtime-status", runtime.status());
                    break;
                }
            }
        });
    }

    fn mark_crashed(&self, message: &str) {
        let mut inner = self.state.lock().expect("runtime lock");
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill();
        }
        inner.state = LlmServerState::Crashed;
        inner.last_error = Some(message.to_string());
    }

    fn emit_status(&self, app: &AppHandle, inner: &RuntimeInner) {
        let _ = app.emit("llm-runtime-status", self.status_from_inner(inner));
    }

    fn status_from_inner(&self, inner: &RuntimeInner) -> LlmStatus {
        let binary_path = config::resolve_llama_server_binary(&self.app_data_dir)
            .map(|p| p.display().to_string());
        let healthy =
            matches!(inner.state, LlmServerState::Running) && self.health_check_sync(inner.port);

        LlmStatus {
            state: inner.state,
            host: inner.host.clone(),
            port: inner.port,
            pid: inner.child.as_ref().map(|c| c.id()),
            healthy,
            model_path: inner.model_path.clone(),
            last_error: inner.last_error.clone(),
            binary_path,
        }
    }
}

use std::path::{Path, PathBuf};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8081;
pub const HEALTH_PATH: &str = "/health";
pub const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";

pub fn models_dir(app_data: &Path) -> PathBuf {
    app_data.join("models")
}

pub fn binaries_dir(app_data: &Path) -> PathBuf {
    app_data.join("bin")
}

pub fn resolve_registry_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(dir).join("../models/registry.json");
        if candidate.exists() {
            return candidate;
        }
    }

    PathBuf::from("models/registry.json")
}

pub fn resolve_llama_server_binary(app_data: &Path) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("LLAMA_SERVER_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let bundled = binaries_dir(app_data).join(if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    });
    if bundled.is_file() {
        return Some(bundled);
    }

    let dev_path = PathBuf::from(r"d:\traeprojects\prdss\bin\llama-server.exe");
    if dev_path.is_file() {
        return Some(dev_path);
    }

    which_llama_server()
}

fn which_llama_server() -> Option<PathBuf> {
    let names = if cfg!(windows) {
        vec!["llama-server.exe", "llama-server"]
    } else {
        vec!["llama-server"]
    };

    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        for name in &names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn server_base_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
}

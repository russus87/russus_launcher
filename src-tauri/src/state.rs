use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub username: String,
    pub auto_check_minutes: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            username: "russus87".to_string(),
            auto_check_minutes: 60,
        }
    }
}

/// Persisted between runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Persisted {
    #[serde(default)]
    pub settings: Settings,
    /// Repo names we have already seen — used to flag genuinely new projects.
    #[serde(default)]
    pub known_repos: Vec<String>,
    /// For platforms without a package manager (macOS): repo -> installed version.
    #[serde(default)]
    pub installed: BTreeMap<String, String>,
    /// repo -> last release tag we notified about (avoid repeat notifications).
    #[serde(default)]
    pub notified: BTreeMap<String, String>,
}

pub struct AppState {
    pub data: Mutex<Persisted>,
    pub path: PathBuf,
}

impl AppState {
    pub fn load() -> Self {
        let path = config_path();
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Persisted>(&s).ok())
            .unwrap_or_default();
        Self {
            data: Mutex::new(data),
            path,
        }
    }

    pub fn save(&self) {
        if let Ok(data) = self.data.lock() {
            if let Some(parent) = self.path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(s) = serde_json::to_string_pretty(&*data) {
                let _ = std::fs::write(&self.path, s);
            }
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("russus-launcher");
    dir.push("state.json");
    dir
}

pub fn cache_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("russus-launcher");
    dir
}

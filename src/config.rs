use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: Uuid,
    pub symbol: String,
    // x, y
    pub position: Option<(i32, i32)>, 
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub locked: bool,
    pub widgets: Vec<WidgetConfig>,
}

impl AppConfig {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("desktop-widget-rs");
        std::fs::create_dir_all(&path).ok();
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        
        // Default config if none exists
        AppConfig {
            locked: false,
            widgets: vec![
                WidgetConfig {
                    id: Uuid::new_v4(),
                    symbol: "AAPL".to_string(),
                    position: None,
                },
            ],
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}

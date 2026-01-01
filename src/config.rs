use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChartConfig {
    pub symbol: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    pub charts: Vec<ChartConfig>,
    #[serde(default = "default_interval")]
    pub update_interval_minutes: u64,
}

fn default_interval() -> u64 {
    30
}

impl AppConfig {
    pub fn load() -> Self {
        if Path::new("config.json").exists() {
            if let Ok(content) = fs::read_to_string("config.json") {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        AppConfig::default()
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write("config.json", content);
        }
    }
}

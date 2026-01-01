use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChartConfig {
    pub symbol: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub charts: Vec<ChartConfig>,
    #[serde(default = "default_interval")]
    pub update_interval_minutes: u64,
}

fn default_interval() -> u64 {
    30
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            charts: Vec::new(),
            update_interval_minutes: default_interval(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        // 1. Try executable directory
        if let Ok(mut path) = env::current_exe() {
            path.pop();
            path.push("config.json");
            if path.exists() {
                 return Self::load_from_path(&path);
            }
        }
        
        // 2. Try current working directory
        let cwd_path = Path::new("config.json");
        if cwd_path.exists() {
            return Self::load_from_path(cwd_path);
        }

        AppConfig::default()
    }

    fn load_from_path(path: &Path) -> Self {
         if let Ok(content) = fs::read_to_string(path) {
             if let Ok(config) = serde_json::from_str(&content) {
                 return config;
             }
         }
         AppConfig::default()
    }

    pub fn save(&self) {
        let mut path = Path::new("config.json").to_path_buf();
        
        if let Ok(mut exe_dist) = env::current_exe() {
            exe_dist.pop();
            exe_dist.push("config.json");
            
            // If it exists in exe dir, use it.
            if exe_dist.exists() {
                path = exe_dist;
            } 
            // Else if it exists in CWD, use it (path is already CWD).
            // (Only if exe_dist doesn't exist)
            else if Path::new("config.json").exists() {
                // keep path as is (CWD)
            }
            // Else, default to creating in exe dir
            else {
                path = exe_dist;
            }
        } 
        
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}

use serde::{Deserialize, Serialize};

pub const PIPE_NAME: &str = r"\\.\pipe\desktop-widget-rs-ipc";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChartData {
    pub id: String,
    pub symbol: String,
    pub timeframe: String,
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigData {
    pub language: String,
    pub update_interval: u64,
    pub auto_start: bool,
    pub use_prereleases: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcMessage {
    GetCharts,
    GetConfig, // Request initial config
    Config(ConfigData), // Response with config
    Charts(Vec<ChartData>),
    AddChart(String),
    DeleteChart(String),
    ToggleChartLock(String, bool),
    SetChartTimeframe(String, String),
    SetLanguage(String),
    SetUpdateInterval(u64),
    SetAutoStart(bool),
    SetUsePrereleases(bool),
    CheckForUpdates,
    PerformUpdate,
    UpdateStatus(crate::common::UpdateStatus),
    Error(String),
    Restart,
    Shutdown, 
}

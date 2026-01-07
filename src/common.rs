use crate::config::ChartConfig;
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::event_loop::ActiveEventLoop;
use yahoo_finance_api as yahoo;

#[derive(Clone, Debug)]
pub enum UpdateStatus {
    Checking,
    Available(String), // version
    UpToDate,
    Error(String),
    Updating,
    Updated(String), // version
}

#[derive(Debug)]
pub enum UserEvent {
    DataLoaded(String, Vec<yahoo::Quote>, String), // Symbol, Quotes, Currency
    Error(String, crate::language::AppError), // Symbol, AppError
    AddChart(String),
    DeleteChart(WindowId),
    OpenSettings,
    ToggleLock(WindowId, bool),
    UpdateInterval(u64),
    ChartTimeframe(WindowId, String),
    LanguageChanged(crate::language::Language),
    CheckForUpdates,
    UpdateStatus(UpdateStatus),
    PerformUpdate,
}

pub trait WindowHandler {
    fn window_id(&self) -> WindowId;
    fn handle_event(&mut self, event: WindowEvent, event_loop: &ActiveEventLoop);
    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>);
    fn redraw(&mut self);
    fn update_data(&mut self, quotes: Vec<yahoo::Quote>, currency: String);
    fn update_active_charts(&mut self, _charts: Vec<(WindowId, String, bool, String)>) {}
    fn get_config(&self) -> Option<ChartConfig> { None }
    fn set_locked(&mut self, _locked: bool) {}
    fn set_timeframe(&mut self, _timeframe: String) {}
    fn refresh(&mut self) {}
    fn tick(&mut self) {}
    fn show_error(&mut self, _message: String) {}
    fn set_language(&mut self, _language: crate::language::Language) {}
    fn has_data(&self) -> bool { true }
    fn update_status(&mut self, _status: UpdateStatus) {}
}

use crate::config::ChartConfig;
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::event_loop::ActiveEventLoop;
use yahoo_finance_api as yahoo;

#[derive(Debug)]
pub enum UserEvent {
    DataLoaded(String, Vec<yahoo::Quote>, String), // Symbol, Quotes, Currency
    Error(String, String), // Symbol, Error Message
    AddChart(String),
    DeleteChart(WindowId),
    OpenSettings,
    ToggleLock(WindowId, bool),
    UpdateInterval(u64),
    ChartTimeframe(WindowId, String),
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
    fn has_data(&self) -> bool { true }
}

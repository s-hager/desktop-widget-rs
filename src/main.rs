mod common;
mod chart;
mod settings;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;
use std::collections::HashMap;
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};
use tray_icon::menu::{Menu, MenuItem, MenuEvent}; // Check MenuEvent usagerEvent, WindowHandler};
use common::{UserEvent, WindowHandler};
use chart::ChartWindow;
use settings::SettingsWindow;

mod config; // Check config module usage

use config::AppConfig;

struct App {
    windows: HashMap<WindowId, Box<dyn WindowHandler>>,
    proxy: EventLoopProxy<UserEvent>,
    tray_icon: Option<TrayIcon>,
    tray_menu: Option<Menu>,
    // Store IDs to manage settings list
    chart_ids: Vec<(WindowId, String, bool, String)>, 
    settings_id: Option<WindowId>,
    settings_menu_id: Option<String>,
    quit_menu_id: Option<String>,
    config: AppConfig,
    dirty: bool,
    last_save_time: std::time::Instant,
    last_auto_refresh: std::time::Instant,
}

impl App {
    fn refresh_settings_window(&mut self) {
        // Filter charts to only include those that have data
        let mut visible_charts = Vec::new();
        for (id, symbol, locked, timeframe) in &self.chart_ids {
            if let Some(handler) = self.windows.get(id) {
                if handler.has_data() {
                    visible_charts.push((*id, symbol.clone(), *locked, timeframe.clone()));
                }
            }
        }

        if let Some(sid) = self.settings_id {
            if let Some(handler) = self.windows.get_mut(&sid) {
                handler.update_active_charts(visible_charts);
            }
        }
    }

    fn save_config(&self) {
        let mut charts = Vec::new();
        for handler in self.windows.values() {
            if handler.has_data() {
                if let Some(config) = handler.get_config() {
                    charts.push(config);
                }
            }
        }
        let app_config = AppConfig { 
            charts,
            update_interval_minutes: self.config.update_interval_minutes,
        };
        app_config.save();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize Tray if not exists
        if self.tray_icon.is_none() {
             let tray_menu = Menu::new();
             let settings_i = MenuItem::new("Settings", true, None);
             let quit_i = MenuItem::new("Quit", true, None);
             
             self.settings_menu_id = Some(settings_i.id().0.clone());
             self.quit_menu_id = Some(quit_i.id().0.clone());

             tray_menu.append(&settings_i).unwrap();
             tray_menu.append(&quit_i).unwrap();

             let icon_rgba = vec![255u8; 32 * 32 * 4]; 
             let icon = Icon::from_rgba(icon_rgba, 32, 32).unwrap();
             
             let tray_icon = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu.clone()))
                .with_icon(icon)
                .with_tooltip("Stock Widget")
                .build()
                .unwrap();

             self.tray_icon = Some(tray_icon);
             self.tray_menu = Some(tray_menu);
        }

        // Open initial charts from config
        if self.windows.is_empty() {
            self.config = AppConfig::load();
            if self.config.charts.is_empty() {
                 let chart = ChartWindow::new(event_loop, self.proxy.clone(), "AAPL".to_string(), None);
                 let id = chart.window_id();
                 self.windows.insert(id, Box::new(chart));
                 self.chart_ids.push((id, "AAPL".to_string(), true, "1M".to_string()));
            } else {
                 for chart_cfg in &self.config.charts {
                     let chart = ChartWindow::new(event_loop, self.proxy.clone(), chart_cfg.symbol.clone(), Some(chart_cfg.clone()));
                     let id = chart.window_id();
                     self.windows.insert(id, Box::new(chart));
                     let tf = chart_cfg.timeframe.clone().unwrap_or("1M".to_string());
                     self.chart_ids.push((id, chart_cfg.symbol.clone(), true, tf));
                 }
            }
         }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            self.windows.remove(&window_id);
            self.chart_ids.retain(|(id, _, _, _)| *id != window_id);
            
            if Some(window_id) == self.settings_id {
                self.settings_id = None;
                // Lock all charts when settings closes
                for entry in &mut self.chart_ids {
                    entry.2 = true; // Set locked to true
                }
                // We need to clone the IDs to iterate and mutate windows
                let ids: Vec<WindowId> = self.windows.keys().cloned().collect(); 
                for id in ids {
                    if let Some(handler) = self.windows.get_mut(&id) {
                         // Only lock chart windows (Settings is already closing/closed, but generic check doesn't hurt)
                         handler.set_locked(true);
                    }
                }
            } else {
                // If a chart closed, update settings list & save
                self.refresh_settings_window();
                self.save_config();
            }
            return;
        }
        // Check for move/resize to trigger save
        if let WindowEvent::Moved(_) | WindowEvent::Resized(_) = event {
            self.dirty = true;
        }

        if let Some(handler) = self.windows.get_mut(&window_id) {
            handler.handle_event(event, event_loop);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
         if self.dirty && self.last_save_time.elapsed() > std::time::Duration::from_millis(500) {
             self.save_config();
             self.dirty = false;
             self.last_save_time = std::time::Instant::now();
         }

         // Auto-Refresh
         let refresh_interval = std::time::Duration::from_secs(self.config.update_interval_minutes * 60);
         if self.last_auto_refresh.elapsed() >= refresh_interval {
             for handler in self.windows.values_mut() {
                 handler.refresh();
             }
             self.last_auto_refresh = std::time::Instant::now();
         }
         
         // Calculate next wake up
         let next_refresh = self.last_auto_refresh + refresh_interval;
         let mut next_wake = next_refresh;

         if self.dirty {
             let next_save = self.last_save_time + std::time::Duration::from_millis(500);
             if next_save < next_wake {
                 next_wake = next_save;
             }
         }
         
         event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));

         use tray_icon::{TrayIconEvent, MouseButton, MouseButtonState};

         while let Ok(event) = MenuEvent::receiver().try_recv() {
             let id = event.id.0;
             if let Some(sid) = &self.settings_menu_id {
                 if id == *sid {
                     let _ = self.proxy.send_event(UserEvent::OpenSettings);
                 }
             }
             if let Some(qid) = &self.quit_menu_id {
                 if id == *qid {
                     self.save_config(); 
                     event_loop.exit();
                 }
             }
         }

         while let Ok(event) = TrayIconEvent::receiver().try_recv() {
             if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                  let _ = self.proxy.send_event(UserEvent::OpenSettings);
             }
         }
    }
    
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.save_config();
    }
    
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
             UserEvent::DataLoaded(symbol, quotes, currency) => {
                 let targets: Vec<WindowId> = self.chart_ids.iter()
                     .filter(|(_, s, _, _)| *s == symbol)
                     .map(|(id, _, _, _)| *id)
                     .collect();
                
                 for id in targets {
                     if let Some(h) = self.windows.get_mut(&id) {
                         h.update_data(quotes.clone(), currency.clone());
                     }
                 }
                 self.refresh_settings_window();
                 self.save_config();
             },
             UserEvent::Error(symbol, e) => {
                 eprintln!("Error fetching data for {}: {}", symbol, e);
                 
                 // Show error in Settings if open
                 if let Some(sid) = self.settings_id {
                     if let Some(handler) = self.windows.get_mut(&sid) {
                         handler.show_error(format!("Error: {}", e)); // Or just "Symbol not found"
                     }
                 }

                 // If this was a new window (no data yet), delete it
                 // We need to find the ID associated with this symbol
                 let target_id = self.chart_ids.iter()
                    .find(|(_, s, _, _)| *s == symbol)
                    .map(|(id, _, _, _)| *id);

                 if let Some(id) = target_id {
                     // Check if it has data
                     let should_delete = if let Some(h) = self.windows.get(&id) {
                         !h.has_data()
                     } else { false };

                     if should_delete {
                         self.windows.remove(&id);
                         self.chart_ids.retain(|(wid, _, _, _)| *wid != id);
                         self.refresh_settings_window();
                         self.save_config();
                     }
                 }
             },
             UserEvent::AddChart(symbol) => {
                 let chart = ChartWindow::new(event_loop, self.proxy.clone(), symbol.clone(), None);
                 let id = chart.window_id();
                 self.windows.insert(id, Box::new(chart));
                 self.chart_ids.push((id, symbol, true, "1M".to_string()));
                 self.refresh_settings_window();
             },
             UserEvent::DeleteChart(id) => {
                 self.windows.remove(&id);
                 self.chart_ids.retain(|(wid, _, _, _)| *wid != id);
                 self.refresh_settings_window();
                 self.save_config();
             },
             UserEvent::ToggleLock(id, locked) => {
                 // Update internal state
                 if let Some(entry) = self.chart_ids.iter_mut().find(|(wid, _, _, _)| *wid == id) {
                     entry.2 = locked;
                 }
                 // Update window
                 if let Some(handler) = self.windows.get_mut(&id) {
                     handler.set_locked(locked);
                 }
                 self.refresh_settings_window();
             },
             UserEvent::UpdateInterval(minutes) => {
                 self.config.update_interval_minutes = minutes;
                 self.last_auto_refresh = std::time::Instant::now(); // Reset timer on change
                 self.save_config();
             },
             UserEvent::ChartTimeframe(id, timeframe) => {
                 if let Some(entry) = self.chart_ids.iter_mut().find(|(wid, _, _, _)| *wid == id) {
                     entry.3 = timeframe.clone();
                 }
                 if let Some(handler) = self.windows.get_mut(&id) {
                     handler.set_timeframe(timeframe);
                 }
                 self.refresh_settings_window();
                 self.save_config();
             },
             UserEvent::OpenSettings => {
                 if self.settings_id.is_none() {
                     let mut settings = SettingsWindow::new(event_loop, self.proxy.clone(), self.config.update_interval_minutes);
                     settings.update_active_charts(self.chart_ids.clone());
                     let id = settings.window_id();
                     self.windows.insert(id, Box::new(settings));
                     self.settings_id = Some(id);
                 } else {
                     if let Some(id) = self.settings_id {
                        if let Some(_w) = self.windows.get(&id) {
                            // Focus or request redraw
                            // w.window().focus_window(); // accessing Window from handler?
                            // WindowHandler doesn't expose Window.
                            // Handlers usually have `focus()` method?
                            // For now just ignore.
                        }
                     }
                 }
             }
        }
    }
}

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let proxy = event_loop.create_proxy();
    
    let mut app = App { 
        windows: HashMap::new(),
        proxy,
        tray_icon: None,
        tray_menu: None,
        chart_ids: Vec::new(),
        settings_id: None,
        settings_menu_id: None,
        quit_menu_id: None,
        config: AppConfig::default(),
        dirty: false,
        last_save_time: std::time::Instant::now(),
        last_auto_refresh: std::time::Instant::now(),
    };
    
    event_loop.run_app(&mut app).unwrap();
}

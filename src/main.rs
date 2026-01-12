#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

const AUM_ID: &str = "desktop-widget-rs";

mod common;
mod chart;
mod settings;
mod language;
mod updater;
mod config;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;
use std::collections::HashMap;
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};
use tray_icon::menu::{Menu, MenuItem, MenuEvent}; 
use common::{UserEvent, WindowHandler, UpdateStatus};
use chart::ChartWindow;
use settings::SettingsWindow;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};
use std::path::Path;
use config::AppConfig;
use language::{TextId, get_text};
use std::os::windows::process::CommandExt;



// TODO: might want to delete as well
fn register_aumid(aum_id: &str, display_name: &str, icon_path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    // HKCU\SOFTWARE\Classes\AppUserModelId\desktop-widget-rs
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(format!(r"SOFTWARE\Classes\AppUserModelId\{}", aum_id))?;

    key.set_value("DisplayName", &display_name)?;

    if let Some(path) = icon_path {
        key.set_value("IconUri", &path.to_string_lossy().to_string())?;
    } else {
        let _ = key.delete_value("IconUri");
    }

    Ok(())
}

struct App {
    windows: HashMap<WindowId, Box<dyn WindowHandler>>,
    proxy: EventLoopProxy<UserEvent>,
    tray_icon: Option<TrayIcon>,
    tray_menu: Option<Menu>,
    // Store IDs to manage settings list
    chart_ids: Vec<(WindowId, String, bool, String)>, 
    settings_id: Option<WindowId>,
    settings_item: Option<MenuItem>,
    quit_item: Option<MenuItem>,
    config: AppConfig,
    dirty: bool,
    last_save_time: std::time::Instant,
    last_auto_refresh: std::time::Instant,
    last_update_check: std::time::Instant,
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
            language: self.config.language,
        };
        app_config.save();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize Tray if not exists
        if self.tray_icon.is_none() {
             // Load config here so we have the correct language for the tray menu
             self.config = AppConfig::load();

             let tray_menu = Menu::new();
             let settings_text = get_text(self.config.language, TextId::SettingsMenu);
             let quit_text = get_text(self.config.language, TextId::Quit);

             let settings_i = MenuItem::new(settings_text, true, None);
             let quit_i = MenuItem::new(quit_text, true, None);
             
             tray_menu.append(&settings_i).unwrap();
             tray_menu.append(&quit_i).unwrap();

             self.settings_item = Some(settings_i);
             self.quit_item = Some(quit_i);

             let icon_rgba = vec![255u8; 32 * 32 * 4]; 
             let icon = Icon::from_rgba(icon_rgba, 32, 32).unwrap();
             
             let tray_icon = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu.clone()))
                .with_icon(icon)
                .with_tooltip(AUM_ID)
                .build()
                .unwrap();

             self.tray_icon = Some(tray_icon);
             self.tray_menu = Some(tray_menu);

             // Check for updates on startup
             let _ = self.proxy.send_event(UserEvent::CheckForUpdates);
        }

        // Open initial charts from config
        if self.windows.is_empty() {
            if self.config.charts.is_empty() {
                 let chart = ChartWindow::new(event_loop, self.proxy.clone(), "AAPL".to_string(), None, self.config.language);
                 let id = chart.window_id();
                 self.windows.insert(id, Box::new(chart));
                 self.chart_ids.push((id, "AAPL".to_string(), true, "1M".to_string()));
            } else {
                 for chart_cfg in &self.config.charts {
                     let chart = ChartWindow::new(event_loop, self.proxy.clone(), chart_cfg.symbol.clone(), Some(chart_cfg.clone()), self.config.language);
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

         // Tick all handlers (for debounce/cache logic)
         for handler in self.windows.values_mut() {
             handler.tick();
         }

         // Auto-Refresh
         let refresh_interval = std::time::Duration::from_secs(self.config.update_interval_minutes * 60);
         if self.last_auto_refresh.elapsed() >= refresh_interval {
             for handler in self.windows.values_mut() {
                 handler.refresh();
             }
             self.last_auto_refresh = std::time::Instant::now();
         }

         // Auto-Update Check (every 30 mins)
         let update_check_interval = std::time::Duration::from_secs(30 * 60);
         if self.last_update_check.elapsed() >= update_check_interval {
             let _ = self.proxy.send_event(UserEvent::CheckForUpdates);
             self.last_update_check = std::time::Instant::now();
         }
         
         // Calculate next wake up
         let next_refresh = self.last_auto_refresh + refresh_interval;
         let mut next_wake = next_refresh;

         let next_update = self.last_update_check + update_check_interval;
         if next_update < next_wake {
             next_wake = next_update;
         }

         if self.dirty {
             let next_save = self.last_save_time + std::time::Duration::from_millis(500);
             if next_save < next_wake {
                 next_wake = next_save;
             }
         }
         
         event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));

         // If we have pending debounces, we might want to wake up sooner.
         // Ideally, handlers should provide "next_wake" hint, but for now we rely on user input waking us 
         // or the next refresh/save interval. To support debounce timeout (e.g. 500ms), we should cap wait time.
         if next_wake > std::time::Instant::now() + std::time::Duration::from_millis(100) {
             event_loop.set_control_flow(ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(100)));
         } else {
             event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));
         }

         use tray_icon::{TrayIconEvent, MouseButton, MouseButtonState};

         while let Ok(event) = MenuEvent::receiver().try_recv() {
             let id = event.id;
             if let Some(item) = &self.settings_item {
                 if id == item.id() {
                     let _ = self.proxy.send_event(UserEvent::OpenSettings);
                 }
             }
             if let Some(item) = &self.quit_item {
                 if id == item.id() {
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
             UserEvent::Error(symbol, app_error) => {
                 let localized_err = language::get_error_text(self.config.language, &app_error);
                 eprintln!("Error fetching data for {}: {}", symbol, localized_err);
                 
                 // Show error in Settings if open
                 if let Some(sid) = self.settings_id {
                     if let Some(handler) = self.windows.get_mut(&sid) {
                         let prefix = get_text(self.config.language, TextId::ErrorPrefix);
                         handler.show_error(format!("{} {}", prefix, localized_err)); 
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
                 let chart = ChartWindow::new(event_loop, self.proxy.clone(), symbol.clone(), None, self.config.language);
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
                     let mut settings = SettingsWindow::new(event_loop, self.proxy.clone(), self.config.update_interval_minutes, self.config.language);
                     settings.update_active_charts(self.chart_ids.clone());
                     let id = settings.window_id();
                     self.windows.insert(id, Box::new(settings));
                     self.settings_id = Some(id);
                 } else {
                     if let Some(id) = self.settings_id {
                        if let Some(_w) = self.windows.get(&id) {
                            // Focus logic
                        }
                     }
                 }
             },
             UserEvent::LanguageChanged(lang) => {
                 self.config.language = lang;
                 for handler in self.windows.values_mut() {
                     handler.set_language(lang);
                 }
                 
                 // Update Tray Menu
                 if let Some(item) = &self.settings_item {
                     item.set_text(get_text(lang, TextId::SettingsMenu));
                 }
                 if let Some(item) = &self.quit_item {
                     item.set_text(get_text(lang, TextId::Quit));
                 }

                 self.save_config();
             },
             UserEvent::CheckForUpdates => {
                 let proxy = self.proxy.clone();
                 // Show checking status immediately
                 if let Some(sid) = self.settings_id {
                     if let Some(handler) = self.windows.get_mut(&sid) {
                        handler.update_status(UpdateStatus::Checking(env!("CARGO_PKG_VERSION").to_string()));
                     }
                 }
                 
                 std::thread::spawn(move || {
                     match updater::check_update() {
                         Ok(Some(release)) => {
                             let _ = proxy.send_event(UserEvent::UpdateStatus(UpdateStatus::Available(release.version)));
                         },
                         Ok(None) => {
                             let _ = proxy.send_event(UserEvent::UpdateStatus(UpdateStatus::UpToDate(env!("CARGO_PKG_VERSION").to_string())));
                         },
                         Err(e) => {
                             println!("Check Update Error: {}", e);
                             let _ = proxy.send_event(UserEvent::UpdateStatus(UpdateStatus::Error(e.to_string())));
                         }
                     }
                 });
             },
             UserEvent::PerformUpdate => {
                 let proxy = self.proxy.clone();
                 // Show updating status
                 if let Some(sid) = self.settings_id {
                     if let Some(handler) = self.windows.get_mut(&sid) {
                         handler.update_status(UpdateStatus::Updating);
                     }
                 }

                 std::thread::spawn(move || {
                     match updater::perform_update() {
                         Ok(version) => {
                             let _ = proxy.send_event(UserEvent::UpdateStatus(UpdateStatus::Updated(version)));
                         },
                         Err(e) => {
                             println!("Perform Update Error: {}", e);
                             let _ = proxy.send_event(UserEvent::UpdateStatus(UpdateStatus::Error(e.to_string())));
                         }
                     }
                 });
             },
             UserEvent::UpdateStatus(status) => {
                 if let UpdateStatus::Available(ref version) = status {
                     if self.settings_id.is_none() {
                         if let Err(e) = updater::show_update_notification(version, AUM_ID, self.proxy.clone(), self.config.language) {
                             eprintln!("Failed to show notification: {}", e);
                         }
                     }
                 }

                 if let Some(sid) = self.settings_id {
                     if let Some(handler) = self.windows.get_mut(&sid) {
                         handler.update_status(status);
                     }
                 }
             },
             UserEvent::RestartApp => {
                 // Spawn a new instance of the application
                 if let Ok(exe_path) = std::env::current_exe() {
                     const CREATE_NO_WINDOW: u32 = 0x08000000;
                     let _ = std::process::Command::new(exe_path)
                         .creation_flags(CREATE_NO_WINDOW)
                         .spawn();
                 }
                 event_loop.exit();
             }
         }
    }
}

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    // Register AUMID in registry to make notifications work
    if let Err(e) = register_aumid(AUM_ID, "Desktop Widget", None) {
        eprintln!("Failed to register AUMID: {:?}", e);
    }
    
    let proxy = event_loop.create_proxy();
    
    let mut app = App { 
        windows: HashMap::new(),
        proxy,
        tray_icon: None,
        tray_menu: None,
        chart_ids: Vec::new(),
        settings_id: None,
        settings_item: None,
        quit_item: None,
        config: AppConfig::default(),
        dirty: false,
        last_save_time: std::time::Instant::now(),
        last_auto_refresh: std::time::Instant::now(),
        last_update_check: std::time::Instant::now(),
    };
    
    event_loop.run_app(&mut app).unwrap();
}

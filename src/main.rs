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

struct App {
    windows: HashMap<WindowId, Box<dyn WindowHandler>>,
    proxy: EventLoopProxy<UserEvent>,
    tray_icon: Option<TrayIcon>,
    #[allow(dead_code)]
    tray_menu: Option<Menu>,
    // Store IDs to manage settings list
    chart_ids: Vec<(WindowId, String)>, 
    settings_id: Option<WindowId>,
    settings_menu_id: Option<String>,
    quit_menu_id: Option<String>,
}

impl App {
    fn refresh_settings_window(&mut self) {
        if let Some(sid) = self.settings_id {
            if let Some(handler) = self.windows.get_mut(&sid) {
                handler.update_active_charts(self.chart_ids.clone());
            }
        }
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

        // Open initial chart if no windows and no settings
        if self.windows.is_empty() {
             // Optional: Don't open chart by default?
             // User asked for "create/delete instances" in settings. 
             // Maybe start with just Tray? Or one default logic.
             // Previous code started with AAPL. Let's keep it.
             // We can check if we want to restore from file later (persistence).
             let chart = ChartWindow::new(event_loop, self.proxy.clone(), "AAPL".to_string());
             let id = chart.window_id();
             self.windows.insert(id, Box::new(chart));
             self.chart_ids.push((id, "AAPL".to_string()));
         }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            self.windows.remove(&window_id);
            self.chart_ids.retain(|(id, _)| *id != window_id);
            
            if Some(window_id) == self.settings_id {
                self.settings_id = None;
            } else {
                // If a chart closed, update settings list
                self.refresh_settings_window();
            }
            return;
        }

        if let Some(handler) = self.windows.get_mut(&window_id) {
            handler.handle_event(event, event_loop);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
    
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
             UserEvent::DataLoaded(symbol, quotes, currency) => {
                 let targets: Vec<WindowId> = self.chart_ids.iter()
                     .filter(|(_, s)| *s == symbol)
                     .map(|(id, _)| *id)
                     .collect();
                
                 for id in targets {
                     if let Some(h) = self.windows.get_mut(&id) {
                         h.update_data(quotes.clone(), currency.clone());
                     }
                 }
             },
             UserEvent::Error(symbol, e) => {
                 eprintln!("Error fetching data for {}: {}", symbol, e);
             },
             UserEvent::AddChart(symbol) => {
                 let chart = ChartWindow::new(event_loop, self.proxy.clone(), symbol.clone());
                 let id = chart.window_id();
                 self.windows.insert(id, Box::new(chart));
                 self.chart_ids.push((id, symbol));
                 self.refresh_settings_window();
             },
             UserEvent::DeleteChart(id) => {
                 self.windows.remove(&id);
                 self.chart_ids.retain(|(wid, _)| *wid != id);
                 self.refresh_settings_window();
             },
             UserEvent::OpenSettings => {
                 if self.settings_id.is_none() {
                     let mut settings = SettingsWindow::new(event_loop, self.proxy.clone());
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
    };
    
    event_loop.run_app(&mut app).unwrap();
}

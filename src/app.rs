use iced::{window, Task, Element, Subscription, Size};
use iced::widget::{container, text};
use std::collections::HashMap;
use uuid::Uuid;
use crate::config::{AppConfig, WidgetConfig};
use crate::stock::{StockClient, StockData};
use crate::components::{widget_view, settings_view};
use std::time::Duration;
use tray_icon::{TrayIcon, TrayIconBuilder, menu::Menu, menu::MenuItem, menu::MenuEvent, TrayIconEvent};
use tray_icon::Icon;

pub struct App {
    config: AppConfig,
    stocks: HashMap<Uuid, Option<StockData>>,
    windows: HashMap<window::Id, WindowType>,
    tray: Option<TrayIcon>,
    settings_input: String,
    settings_menu_id: String,
    quit_menu_id: String,
}

#[derive(Debug, Clone)]
pub enum WindowType {
    Widget(Uuid),
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    StockUpdate(Uuid, Option<StockData>),
    WidgetWindowOpened(window::Id, Uuid),
    SettingsWindowOpened(window::Id),
    Tick,
    TrayEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
    OpenSettings,
    CloseSettings,
    RequestClose(window::Id),
    SettingsAction(settings_view::SettingsMessage),
    DragWindow(window::Id),
    UnusedTick,
    None,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = AppConfig::load();
        let mut tasks = Vec::new();
        let mut initial_windows = HashMap::new();

        // Start Tasks to open windows
        for widget in &config.widgets {
            let id = widget.id;
            let (x, y) = widget.position.unwrap_or((100, 100)); // Default pos
            
            // Initial fetch
            tasks.push(Task::perform(
                fetch_stock(widget.symbol.clone(), id),
                move |(uid, data)| Message::StockUpdate(uid, data),
            ));

            // Open Window
            // Open Window
            let (window_id, open_task) = window::open(window::Settings {
                position: window::Position::Specific(iced::Point { x: x as f32, y: y as f32 }),
                size: Size::new(300.0, 150.0),
                decorations: false,
                transparent: true,
                level: window::Level::Normal,
                icon: None,
                platform_specific: window::settings::PlatformSpecific {
                    skip_taskbar: true,
                    ..Default::default()
                },
                ..Default::default()
            });
            
            initial_windows.insert(window_id, WindowType::Widget(id));
            
            tasks.push(open_task.map(move |id_opts| Message::WidgetWindowOpened(id_opts, id)));
        }

        // Setup Tray

        let tray_menu = Menu::new();
        let settings_item = MenuItem::new("Settings", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        tray_menu.append_items(&[&settings_item, &quit_item]).unwrap();
        
        let settings_menu_id = settings_item.id().0.clone();
        let quit_menu_id = quit_item.id().0.clone();
        
        let icon = Icon::from_rgba(vec![255, 0, 0, 255], 1, 1).unwrap();
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Stock Widget")
            .with_icon(icon)
            .build()
            .unwrap();

        (
            App {
                config,
                stocks: HashMap::new(),
                windows: initial_windows,
                tray: Some(tray),
                settings_input: String::new(),
                settings_menu_id,
                quit_menu_id,
            },
            Task::batch(tasks),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StockUpdate(id, data) => {
                self.stocks.insert(id, data);
                Task::none()
            }
            Message::WidgetWindowOpened(window_id, widget_id) => {
                self.windows.insert(window_id, WindowType::Widget(widget_id));
                Task::none()
            }
            Message::SettingsWindowOpened(window_id) => {
                self.windows.insert(window_id, WindowType::Settings);
                Task::none()
            }
            Message::Tick => {
                // Refresh all stocks
                let mut tasks = Vec::new();
                for widget in &self.config.widgets {
                    tasks.push(Task::perform(
                        fetch_stock(widget.symbol.clone(), widget.id),
                        move |(uid, data)| Message::StockUpdate(uid, data),
                    ));
                }
                Task::batch(tasks)
            }
            Message::TrayEvent(e) => {
               if let TrayIconEvent::Click { button, .. } = e {
                   if button == tray_icon::MouseButton::Left {
                       return Task::done(Message::OpenSettings);
                   }
               }
               Task::none()
            }
            Message::MenuEvent(e) => {
                if e.id.0 == self.quit_menu_id {
                    return iced::exit();
                } else if e.id.0 == self.settings_menu_id {
                    return Task::done(Message::OpenSettings);
                }
                Task::none()
            }
            Message::OpenSettings => {
                // Check if already open
                if self.windows.values().any(|t| matches!(t, WindowType::Settings)) {
                    return Task::none();
                }
                
                let (win_id, open_task) = window::open(window::Settings {
                    size: Size::new(400.0, 500.0),
                    decorations: true,
                    ..Default::default()
                });
                self.windows.insert(win_id, WindowType::Settings);
                open_task.map(Message::SettingsWindowOpened)
            }
            Message::CloseSettings => {
                // Find settings window
                if let Some((&id, _)) = self.windows.iter().find(|(_, t)| matches!(t, WindowType::Settings)) {
                    return window::close(id);
                }
                Task::none()
            }
            Message::RequestClose(id) => {
                 self.windows.remove(&id);
                 window::close(id)
            }
            Message::DragWindow(id) => {
                if !self.config.locked {
                    return window::drag(id);
                }
                Task::none()
            }
            Message::SettingsAction(action) => {
                match action {
                    settings_view::SettingsMessage::ToggleLock(locked) => {
                        self.config.locked = locked;
                        self.config.save();
                    }
                    settings_view::SettingsMessage::InputChanged(val) => {
                        self.settings_input = val;
                    }
                    settings_view::SettingsMessage::Add(symbol) => {
                        if !symbol.is_empty() {
                            let symbol = symbol.to_uppercase();
                            let new_widget = WidgetConfig {
                                id: Uuid::new_v4(),
                                symbol: symbol.clone(),
                                position: None,
                            };
                            self.config.widgets.push(new_widget.clone());
                            self.config.save();
                            self.settings_input.clear();
                            
                            // Spawn logic
                             let (win_id, open_task) = window::open(window::Settings {
                                    size: Size::new(300.0, 150.0),
                                    decorations: false,
                                    transparent: true,
                                    platform_specific: window::settings::PlatformSpecific {
                                        skip_taskbar: true,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                });
                                self.windows.insert(win_id, WindowType::Widget(new_widget.id));

                                return Task::batch(vec![
                                    Task::perform(
                                        fetch_stock(new_widget.symbol.clone(), new_widget.id),
                                        move |(uid, data)| Message::StockUpdate(uid, data),
                                    ),
                                    open_task.map(move |id| Message::WidgetWindowOpened(id, new_widget.id))
                                ]);
                        }
                    }
                    settings_view::SettingsMessage::Remove(idx) => {
                        if idx < self.config.widgets.len() {
                            let removed = self.config.widgets.remove(idx);
                            self.config.save();
                            // Find validity
                            if let Some((&wid, _)) = self.windows.iter().find(|(_, t)| matches!(t, WindowType::Widget(uid) if *uid == removed.id)) {
                                self.windows.remove(&wid);
                                return window::close(wid);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::UnusedTick => Task::none(),
            Message::None => Task::none(),
        }
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if let Some(wtype) = self.windows.get(&window_id) {
            match wtype {
                WindowType::Settings => {
                    settings_view::view(&self.config, &self.settings_input)
                        .map(Message::SettingsAction)
                }
                WindowType::Widget(uid) => {
                    let data = self.stocks.get(uid).and_then(|o| o.as_ref());
                    widget_view::view(data, 300, 150)
                        .map(move |msg| match msg {
                            widget_view::WidgetMessage::StartDrag => Message::DragWindow(window_id),
                        })
                }
            }
        } else {
            text("Loading...").into()
        }
    }
    
    pub fn theme(&self, _id: window::Id) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn title(&self, _id: window::Id) -> String {
        "Stock Widget".to_string()
    }
}


async fn fetch_stock(symbol: String, id: Uuid) -> (Uuid, Option<StockData>) {
    // In real app we might cache or rate limit
    let data = StockClient::fetch_quote(&symbol).await;
    (id, data)
}

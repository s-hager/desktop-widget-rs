use winit::window::{Window, WindowLevel};
use winit::window::WindowId;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::event::{WindowEvent, ElementState, MouseButton};
use winit::keyboard::{Key, NamedKey};
use std::rc::Rc;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use crate::common::{WindowHandler, UserEvent, UpdateStatus};
use yahoo_finance_api as yahoo;
use auto_launch::AutoLaunchBuilder;
use std::env;
use crate::language::{Language, TextId, get_text};


pub struct SettingsWindow {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    // #[allow(dead_code)]
    context: Context<Rc<Window>>,
    proxy: EventLoopProxy<UserEvent>,
    
    // UI State
    input_text: String,
    is_focused: bool,
    active_charts: Vec<(WindowId, String, bool, String)>, // (Id, Symbol, Locked, Timeframe)
    cursor_pos: (f64, f64),
    current_interval: u64,
    startup_enabled: bool,
    error_message: Option<String>,
    language: Language,
    update_status: Option<UpdateStatus>,
}

impl SettingsWindow {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<UserEvent>, initial_interval: u64, initial_language: Language) -> Self {
        let title = get_text(initial_language, TextId::SettingsTitle);
        let window_attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(450.0, 480.0))
            .with_resizable(false); 

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        
        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
             surface.resize(width, height).unwrap();
        }

        let startup_enabled = if let Ok(exe_path) = env::current_exe() {
            if let Some(path_str) = exe_path.to_str() {
                 let auto = AutoLaunchBuilder::new()
                    .set_app_name("DesktopStockWidget")
                    .set_app_path(path_str)
                    .set_use_launch_agent(false) 
                    .build();
                 if let Ok(auto) = auto {
                     auto.is_enabled().unwrap_or(false)
                 } else { false }
            } else { false }
        } else { false };

        Self {
            window,
            surface,
            context,
            proxy,
            input_text: "".to_string(),
            is_focused: false,
            active_charts: Vec::new(),
            cursor_pos: (0.0, 0.0),
            current_interval: initial_interval,
            startup_enabled,
            error_message: None,
            language: initial_language,
            update_status: None,
        }
    }



    pub fn update_active_charts(&mut self, charts: Vec<(WindowId, String, bool, String)>) {
        self.active_charts = charts;
        self.window.request_redraw();
    }
}

impl WindowHandler for SettingsWindow {
    fn window_id(&self) -> WindowId {
        self.window.id()
    }

    fn update_data(&mut self, _quotes: Vec<yahoo::Quote>, _currency: String) {
        // Not used
    }

    fn update_active_charts(&mut self, charts: Vec<(WindowId, String, bool, String)>) {
        self.active_charts = charts;
        self.window.request_redraw();
    }

    fn show_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.window.request_redraw();
    }

    fn set_language(&mut self, language: Language) {
        self.language = language;
        self.window.set_title(get_text(language, TextId::SettingsTitle));
        self.window.request_redraw();
    }

    fn update_status(&mut self, status: UpdateStatus) {
        self.update_status = Some(status);
        self.window.request_redraw();
    }


    fn handle_event(&mut self, event: WindowEvent, _event_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x, position.y);
                self.window.request_redraw(); // For hover effects
            },
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let (x, y) = self.cursor_pos;
                // Hit test input box
                if x >= 20.0 && x <= 220.0 && y >= 50.0 && y <= 80.0 {
                    self.is_focused = true;
                } else {
                    self.is_focused = false;
                }

                // Hit test Add button
                if x >= 230.0 && x <= 290.0 && y >= 50.0 && y <= 80.0 {
                    if !self.input_text.is_empty() {
                        let _ = self.proxy.send_event(UserEvent::AddChart(self.input_text.clone()));
                        self.input_text.clear();
                    }
                }

                // Hit test Delete buttons
                for (i, (id, _, locked, timeframe)) in self.active_charts.iter().enumerate() {
                    let btn_y = 130 + (i as i32 * 30);
                    // Delete Button (x: 220-260)
                    if x >= 220.0 && x <= 260.0 && y >= btn_y as f64 && y <= (btn_y + 25) as f64 {
                        let _ = self.proxy.send_event(UserEvent::DeleteChart(*id));
                    }
                    
                    // Lock Button (x: 270-310)
                    if x >= 270.0 && x <= 310.0 && y >= btn_y as f64 && y <= (btn_y + 25) as f64 {
                         let _ = self.proxy.send_event(UserEvent::ToggleLock(*id, !locked));
                    }

                    // Timeframe Button (x: 320-370)
                    if x >= 320.0 && x <= 370.0 && y >= btn_y as f64 && y <= (btn_y + 25) as f64 {
                        let next_tf = match timeframe.as_str() {
                            "1D" => "1W",
                            "1W" => "1M",
                            "1M" => "3M",
                            "3M" => "6M",
                            "6M" => "1Y",
                            "1Y" => "YTD",
                            "YTD" => "1D",
                            _ => "1M",
                        };
                        let _ = self.proxy.send_event(UserEvent::ChartTimeframe(*id, next_tf.to_string()));
                    }
                }
                
                // Hit test Lock Button (x: 300-360) already handled in loop

                // Hit test Interval Buttons
                // Label at y=360? Bottom of window is 400.
                
                // Auto Startup Toggle (y=320)
                let toggle_y = 320.0;
                if x >= 280.0 && x <= 330.0 && y >= toggle_y && y <= toggle_y + 20.0 {
                    self.startup_enabled = !self.startup_enabled;
                    
                    if let Ok(exe_path) = env::current_exe() {
                        if let Some(path_str) = exe_path.to_str() {
                             let auto = AutoLaunchBuilder::new()
                                .set_app_name("DesktopStockWidget")
                                .set_app_path(path_str)
                                .build();
                             
                             if let Ok(auto) = auto {
                                 if self.startup_enabled {
                                     let _ = auto.enable();
                                 } else {
                                     let _ = auto.disable();
                                 }
                             }
                        }
                    }
                    self.window.request_redraw();
                }

                let footer_y = 360.0;
                
                // Minus
                if x >= 280.0 && x <= 310.0 && y >= footer_y && y <= footer_y + 25.0 {
                    if self.current_interval > 5 {
                        self.current_interval -= 5;
                        let _ = self.proxy.send_event(UserEvent::UpdateInterval(self.current_interval));
                    }
                }
                // Plus
                if x >= 320.0 && x <= 350.0 && y >= footer_y && y <= footer_y + 25.0 {
                    self.current_interval += 5;
                    let _ = self.proxy.send_event(UserEvent::UpdateInterval(self.current_interval));
                }

                // Language Toggle (Top Right: x=340, y=10)
                if x >= 390.0 && x <= 440.0 && y >= 10.0 && y <= 35.0 {
                     let _ = self.proxy.send_event(UserEvent::LanguageChanged(self.language.next()));
                }

                // Update UI (Footer area y=400+)
                let update_y = 400.0;
                // Check Button (20-150)
                if x >= 20.0 && x <= 150.0 && y >= update_y && y <= update_y + 25.0 {
                     if !matches!(self.update_status, Some(UpdateStatus::Checking) | Some(UpdateStatus::Updating) | Some(UpdateStatus::Updated(_))) {
                        let _ = self.proxy.send_event(UserEvent::CheckForUpdates);
                     }
                }
                
                // Update/Restart Button (160-290) - Only clickable if available or updated
                if x >= 160.0 && x <= 290.0 && y >= update_y && y <= update_y + 25.0 {
                    match &self.update_status {
                        Some(UpdateStatus::Available(_)) => {
                             let _ = self.proxy.send_event(UserEvent::PerformUpdate);
                        },
                        Some(UpdateStatus::Updated(_)) => {
                             let _ = self.proxy.send_event(UserEvent::RestartApp);
                        },
                        _ => {}
                    }
                }
                
                self.window.request_redraw();
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if self.is_focused && event.state == ElementState::Pressed {
                    if let Some(txt) = event.text {
                         if txt == "\u{8}" { // Backspace
                             self.input_text.pop();
                         } else if !txt.chars().any(|c| c.is_control()) {
                             self.input_text.push_str(&txt.to_uppercase());
                         }
                         self.error_message = None; // Clear error on typing
                         self.window.request_redraw();
                    }
                    match event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            self.input_text.pop();
                        }
                        Key::Named(NamedKey::Enter) => {
                             if !self.input_text.is_empty() {
                                let _ = self.proxy.send_event(UserEvent::AddChart(self.input_text.clone()));
                                self.input_text.clear();
                            }
                        }
                        _ => {}
                    }
                    self.window.request_redraw();
                }
            },
            WindowEvent::RedrawRequested => {
                self.redraw();
            },
            _ => (),
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
            self.surface.resize(width, height).unwrap();
            self.window.request_redraw();
        }
    }

    fn redraw(&mut self) {
         if let Ok(mut buffer) = self.surface.buffer_mut() {
            let width = buffer.width().get();
            let height = buffer.height().get();
            let mut pixel_buffer = vec![0u32; (width * height) as usize];
            
            {  
                // We'll wrap pixel_buffer in a way Plotters can use, OR manually draw.
                // Plotters Bitmap backend expects u8 slice RGB.
                let mut rgb_buffer = vec![0u8; (width * height * 3) as usize];
                {
                    let root = BitMapBackend::with_buffer(&mut rgb_buffer[..], (width, height)).into_drawing_area();
                    root.fill(&RGBColor(30, 30, 30)).unwrap(); // Dark Grey BG

                    let font = ("sans-serif", 20).into_font();
                    
                    // Title
                    root.draw_text(get_text(self.language, TextId::SettingsTitle), &("sans-serif", 25).into_font().color(&WHITE), (20, 10)).unwrap();

                    // Language Toggle
                    let lang_hover = self.cursor_pos.0 >= 390.0 && self.cursor_pos.0 <= 440.0 && self.cursor_pos.1 >= 10.0 && self.cursor_pos.1 <= 35.0;
                    let lang_bg = if lang_hover { RGBColor(100, 100, 100) } else { RGBColor(60, 60, 60) };
                    root.draw(&Rectangle::new([(390, 10), (440, 35)], lang_bg.filled())).unwrap();
                    root.draw_text(self.language.as_str(), &("sans-serif", 18).into_font().color(&WHITE), (400, 13)).unwrap();

                    // Input Label
                    // root.draw_text("New Symbol:", &font.clone().color(&WHITE), (20, 50)).unwrap();

                    // Input Box
                    let input_bg = if self.is_focused { WHITE } else { RGBColor(200, 200, 200) };
                    root.draw(&Rectangle::new([(20, 50), (220, 80)], input_bg.filled())).unwrap();
                    root.draw_text(&self.input_text, &font.clone().color(&BLACK), (25, 55)).unwrap();

                    // Add Button
                    let add_hover = self.cursor_pos.0 >= 230.0 && self.cursor_pos.0 <= 290.0 && self.cursor_pos.1 >= 50.0 && self.cursor_pos.1 <= 80.0;
                    let add_color = if add_hover { RGBColor(50, 150, 255) } else { RGBColor(0, 100, 200) }; 
                    root.draw(&Rectangle::new([(230, 50), (290, 80)], add_color.filled())).unwrap();
                    root.draw_text(get_text(self.language, TextId::AddButton), &font.clone().color(&WHITE), (245, 55)).unwrap();

                    // Error Message
                    if let Some(err) = &self.error_message {
                        // If error starts with "Error:", we might want to localize the prefix if we construct it here.
                        // But usually it's constructed elsewhere. 
                        // For now just print as is.
                        root.draw_text(err, &("sans-serif", 15).into_font().color(&RED), (20, 85)).unwrap();
                    }

                    // List Header
                    root.draw_text(get_text(self.language, TextId::ActiveCharts), &font.clone().color(&WHITE), (20, 100)).unwrap();

                    // List
                    for (i, (_id, symbol, locked, timeframe)) in self.active_charts.iter().enumerate() {
                         let y = 130 + (i as i32 * 30);
                         root.draw_text(symbol, &font.clone().color(&WHITE), (20, y)).unwrap();
                         
                         // Delete Button
                         let del_hover = self.cursor_pos.0 >= 220.0 && self.cursor_pos.0 <= 260.0 && self.cursor_pos.1 >= y as f64 && self.cursor_pos.1 <= (y + 25) as f64;
                         let del_color = if del_hover { RGBColor(255, 50, 50) } else { RGBColor(200, 0, 0) };
                         root.draw(&Rectangle::new([(220, y), (260, y + 25)], del_color.filled())).unwrap();
                         root.draw_text(get_text(self.language, TextId::DeleteButton), &("sans-serif", 15).into_font().color(&WHITE), (225, y + 5)).unwrap();
                         
                         // Lock Button
                         let lock_hover = self.cursor_pos.0 >= 270.0 && self.cursor_pos.0 <= 310.0 && self.cursor_pos.1 >= y as f64 && self.cursor_pos.1 <= (y + 25) as f64;
                         
                         let lock_color = if *locked {
                             // Locked: Grayish
                             if lock_hover { RGBColor(100, 100, 100) } else { RGBColor(80, 80, 80) }
                         } else {
                             // Unlocked: Yellowish
                             if lock_hover { RGBColor(255, 200, 50) } else { RGBColor(200, 150, 0) }
                         };
                         
                         root.draw(&Rectangle::new([(270, y), (310, y + 25)], lock_color.filled())).unwrap();
                         
                         // Draw Padlock Icon
                         let icon_color = WHITE;
                         let bx = 282; // Body X (270 + 12)
                         let by = y + 10; // Body Y
                         
                         // Body: 16x11 Rect
                         root.draw(&Rectangle::new([(bx, by), (bx + 16, by + 11)], icon_color.filled())).unwrap();
                         
                         // Shackle
                         let sx = bx + 2;
                         let sy = by;
                         let sw = 12; // Shackle width
                         let sh = 6;  // Shackle height (above body)
                         
                         let outline_color = icon_color; 
                         
                         if *locked {
                             // Closed Shackle
                             let points = vec![
                                 (sx, sy),           // Left connection
                                 (sx, sy - sh),      // Left top
                                 (sx + sw, sy - sh), // Right top
                                 (sx + sw, sy),      // Right connection
                             ];
                             root.draw(&PathElement::new(points, outline_color.stroke_width(2))).unwrap();
                         } else {
                             // Open Shackle (Lifted on right, or gap)
                             let points = vec![
                                 (sx, sy),           // Left connection
                                 (sx, sy - sh),      // Left top
                                 (sx + sw, sy - sh), // Right top
                                 (sx + sw, sy - 3),  // Right tip (gap from body)
                             ];
                             root.draw(&PathElement::new(points, outline_color.stroke_width(2))).unwrap();
                         }

                         // Timeframe Button
                         let tf_hover = self.cursor_pos.0 >= 320.0 && self.cursor_pos.0 <= 370.0 && self.cursor_pos.1 >= y as f64 && self.cursor_pos.1 <= (y + 25) as f64;
                         let tf_color = if tf_hover { RGBColor(50, 150, 255) } else { RGBColor(0, 100, 200) };
                         root.draw(&Rectangle::new([(320, y), (370, y + 25)], tf_color.filled())).unwrap();
                         
                         // Center text
                         let tf_text = timeframe.as_str();
                         let (tf_w, _) = font.box_size(tf_text).unwrap();
                         let tf_x = 320 + (50 - tf_w as i32) / 2;
                         root.draw_text(tf_text, &font.clone().color(&WHITE), (tf_x, y + 3)).unwrap();
                    }

                    // Auto Startup Toggle
                    let toggle_y = 320;
                    root.draw_text(get_text(self.language, TextId::AutoStartup), &font.clone().color(&WHITE), (20, toggle_y + 3)).unwrap();
                    
                    // Toggle Switch Background (Rounded Rect)
                    let toggle_rect_color = if self.startup_enabled { RGBColor(0, 200, 100) } else { RGBColor(80, 80, 80) };
                    // 50px wide, 20px high
                    let tx = 280;
                    root.draw(&Rectangle::new([(tx, toggle_y), (tx + 50, toggle_y + 20)], toggle_rect_color.filled())).unwrap();
                    
                    // Knob
                    let knob_x = if self.startup_enabled { tx + 30 } else { tx };
                    root.draw(&Rectangle::new([(knob_x, toggle_y), (knob_x + 20, toggle_y + 20)], WHITE.filled())).unwrap();


                    // Interval Control Footer
                    let footer_y = 360;
                    root.draw_text(get_text(self.language, TextId::UpdateInterval), &font.clone().color(&WHITE), (20, footer_y + 3)).unwrap();
                    
                    // Value
                    root.draw_text(&format!("{}", self.current_interval), &font.clone().color(&WHITE), (240, footer_y + 3)).unwrap();

                    // - Button
                    let min_hover = self.cursor_pos.0 >= 280.0 && self.cursor_pos.0 <= 310.0 && self.cursor_pos.1 >= footer_y as f64 && self.cursor_pos.1 <= (footer_y + 25) as f64;
                    let min_color = if min_hover { RGBColor(100, 100, 100) } else { RGBColor(80, 80, 80) };
                    root.draw(&Rectangle::new([(280, footer_y), (310, footer_y + 25)], min_color.filled())).unwrap();
                    root.draw_text("-", &font.clone().color(&WHITE), (290, footer_y + 3)).unwrap();

                    // + Button
                    let plus_hover = self.cursor_pos.0 >= 320.0 && self.cursor_pos.0 <= 350.0 && self.cursor_pos.1 >= footer_y as f64 && self.cursor_pos.1 <= (footer_y + 25) as f64;
                    let plus_color = if plus_hover { RGBColor(100, 100, 100) } else { RGBColor(80, 80, 80) };
                    root.draw(&Rectangle::new([(320, footer_y), (350, footer_y + 25)], plus_color.filled())).unwrap();
                    root.draw_text("+", &font.clone().color(&WHITE), (330, footer_y + 3)).unwrap();

                    // Update UI
                    let update_y = 400;
                    // Check Button
                    let check_hover = self.cursor_pos.0 >= 20.0 && self.cursor_pos.0 <= 150.0 && self.cursor_pos.1 >= update_y as f64 && self.cursor_pos.1 <= (update_y + 25) as f64;
                    let check_color = if check_hover { RGBColor(100, 100, 100) } else { RGBColor(80, 80, 80) };
                    root.draw(&Rectangle::new([(20, update_y), (150, update_y + 25)], check_color.filled())).unwrap();
                    root.draw_text(get_text(self.language, TextId::UpdateCheck), &("sans-serif", 15).into_font().color(&WHITE), (25, update_y + 5)).unwrap();

                    // Status / Action
                    if let Some(status) = &self.update_status {
                        let status_text = match status {
                            UpdateStatus::Checking => get_text(self.language, TextId::UpdateChecking),
                            UpdateStatus::UpToDate => get_text(self.language, TextId::UpdateUpToDate),
                            UpdateStatus::Available(v) => {
                                // Draw Update Button
                                let btn_hover = self.cursor_pos.0 >= 160.0 && self.cursor_pos.0 <= 290.0 && self.cursor_pos.1 >= update_y as f64 && self.cursor_pos.1 <= (update_y + 25) as f64;
                                let btn_color = if btn_hover { RGBColor(50, 200, 50) } else { RGBColor(0, 150, 0) };
                                root.draw(&Rectangle::new([(160, update_y), (290, update_y + 25)], btn_color.filled())).unwrap();
                                root.draw_text(get_text(self.language, TextId::UpdateBtnNow), &("sans-serif", 15).into_font().color(&WHITE), (165, update_y + 5)).unwrap();
                                
                                // Return version string to print next to it
                                v.as_str()
                            },
                            UpdateStatus::Updating => get_text(self.language, TextId::UpdateUpdating),
                            UpdateStatus::Updated(_) => {
                                 // Draw Restart Button (Reuse same area/style as Update button)
                                let btn_hover = self.cursor_pos.0 >= 160.0 && self.cursor_pos.0 <= 290.0 && self.cursor_pos.1 >= update_y as f64 && self.cursor_pos.1 <= (update_y + 25) as f64;
                                let btn_color = if btn_hover { RGBColor(50, 200, 50) } else { RGBColor(0, 150, 0) };
                                root.draw(&Rectangle::new([(160, update_y), (290, update_y + 25)], btn_color.filled())).unwrap();
                                root.draw_text(get_text(self.language, TextId::UpdateRestart), &("sans-serif", 15).into_font().color(&WHITE), (165, update_y + 5)).unwrap();
                                "" // No extra text
                            },
                            UpdateStatus::Error(_) => get_text(self.language, TextId::UpdateError),
                        };
                        
                        // If it's not the button case (Available or Updated), draw text
                        if !matches!(status, UpdateStatus::Available(_) | UpdateStatus::Updated(_)) {
                            root.draw_text(status_text, &("sans-serif", 15).into_font().color(&WHITE), (160, update_y + 5)).unwrap();
                        } else if matches!(status, UpdateStatus::Available(_)) {
                             // Draw new version info
                             let current_ver = env!("CARGO_PKG_VERSION");
                             root.draw_text(&format!("v{} -> v{}", current_ver, status_text), &("sans-serif", 12).into_font().color(&WHITE), (300, update_y + 8)).unwrap();
                        } else if matches!(status, UpdateStatus::Updated(_)) {
                             // Draw transition even when updated
                             let current_ver = env!("CARGO_PKG_VERSION");
                             // status_text for Updated is empty currently (my bad, check common.rs), actually Updated(String) holds the version!
                             // But wait, my get_text logic for Updated returns "Restart app!"?
                             // Ah, status_text variable holds the RESULT of get_text.
                             // I need to access the version string directly from the match above or the status object.
                             // Let's rely on retrieving it again.
                             if let UpdateStatus::Updated(v) = status {
                                  root.draw_text(&format!("v{} -> v{}", current_ver, v), &("sans-serif", 12).into_font().color(&WHITE), (300, update_y + 8)).unwrap();
                             }
                        }
                    } else {
                        // No status, just show current version
                        let current_ver = env!("CARGO_PKG_VERSION");
                        root.draw_text(&format!("v{}", current_ver), &("sans-serif", 12).into_font().color(&WHITE), (320, update_y + 8)).unwrap();
                    }
                }

                // Copy RGB to u32 0RGB
                for (i, chunk) in rgb_buffer.chunks(3).enumerate() {
                    if i < pixel_buffer.len() {
                        pixel_buffer[i] = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
                    }
                }
            }

            buffer.copy_from_slice(&pixel_buffer);
            buffer.present().ok();
         }
    }
}

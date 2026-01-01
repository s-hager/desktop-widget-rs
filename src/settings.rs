use winit::window::Window;
use winit::window::WindowId;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::event::{WindowEvent, ElementState, MouseButton};
use winit::keyboard::{Key, NamedKey};
use std::rc::Rc;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use crate::common::{WindowHandler, UserEvent};
use yahoo_finance_api as yahoo;



pub struct SettingsWindow {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    // #[allow(dead_code)]
    context: Context<Rc<Window>>,
    proxy: EventLoopProxy<UserEvent>,
    
    // UI State
    input_text: String,
    is_focused: bool,
    active_charts: Vec<(WindowId, String, bool)>, // (Id, Symbol, Locked)
    cursor_pos: (f64, f64),
    current_interval: u64,
}

impl SettingsWindow {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<UserEvent>, initial_interval: u64) -> Self {
        let window_attributes = Window::default_attributes()
            .with_title("Settings")
            .with_inner_size(winit::dpi::LogicalSize::new(400.0, 400.0))
            .with_resizable(false); 

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        
        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
             surface.resize(width, height).unwrap();
        }

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
        }
    }

    pub fn update_active_charts(&mut self, charts: Vec<(WindowId, String, bool)>) {
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

    fn update_active_charts(&mut self, charts: Vec<(WindowId, String, bool)>) {
        self.active_charts = charts;
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
                for (i, (id, _, locked)) in self.active_charts.iter().enumerate() {
                    let btn_y = 130 + (i as i32 * 30);
                    // Delete Button (x: 230-290)
                    if x >= 230.0 && x <= 290.0 && y >= btn_y as f64 && y <= (btn_y + 25) as f64 {
                        let _ = self.proxy.send_event(UserEvent::DeleteChart(*id));
                    }
                    
                    // Lock Button (x: 300-360)
                    if x >= 300.0 && x <= 360.0 && y >= btn_y as f64 && y <= (btn_y + 25) as f64 {
                         let _ = self.proxy.send_event(UserEvent::ToggleLock(*id, !locked));
                    }
                }
                
                // Hit test Lock Button (x: 300-360) already handled in loop

                // Hit test Interval Buttons
                // Label at y=360? Bottom of window is 400.
                // - Button: 280, 360, 30x25
                // + Button: 350, 360, 30x25
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
                
                self.window.request_redraw();
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if self.is_focused && event.state == ElementState::Pressed {
                    if let Some(text) = event.text {
                         // Filter control chars
                         if !text.chars().any(|c| c.is_control()) {
                            self.input_text.push_str(&text.to_uppercase());
                         }
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
                    root.draw_text("Settings", &("sans-serif", 25).into_font().color(&WHITE), (20, 10)).unwrap();

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
                    root.draw_text("Add", &font.clone().color(&WHITE), (245, 55)).unwrap();

                    // List Header
                    root.draw_text("Active Charts:", &font.clone().color(&WHITE), (20, 100)).unwrap();

                    // List
                    for (i, (_id, symbol, locked)) in self.active_charts.iter().enumerate() {
                         let y = 130 + (i as i32 * 30);
                         root.draw_text(symbol, &font.clone().color(&WHITE), (20, y)).unwrap();
                         
                         // Delete Button
                         let del_hover = self.cursor_pos.0 >= 230.0 && self.cursor_pos.0 <= 290.0 && self.cursor_pos.1 >= y as f64 && self.cursor_pos.1 <= (y + 25) as f64;
                         let del_color = if del_hover { RGBColor(255, 50, 50) } else { RGBColor(200, 0, 0) };
                         root.draw(&Rectangle::new([(230, y), (290, y + 25)], del_color.filled())).unwrap();
                         root.draw_text("Del", &font.clone().color(&WHITE), (245, y + 3)).unwrap();
                         
                         // Lock Button
                         let lock_hover = self.cursor_pos.0 >= 300.0 && self.cursor_pos.0 <= 360.0 && self.cursor_pos.1 >= y as f64 && self.cursor_pos.1 <= (y + 25) as f64;
                         
                         let lock_color = if *locked {
                             // Locked: Grayish
                             if lock_hover { RGBColor(100, 100, 100) } else { RGBColor(80, 80, 80) }
                         } else {
                             // Unlocked: Yellowish
                             if lock_hover { RGBColor(255, 200, 50) } else { RGBColor(200, 150, 0) }
                         };
                         
                         root.draw(&Rectangle::new([(300, y), (360, y + 25)], lock_color.filled())).unwrap();
                         
                         // Draw Padlock Icon
                         let icon_color = WHITE;
                         let bx = 322; // Body X
                         let by = y + 10; // Body Y
                         
                         // Body: 16x11 Rect
                         root.draw(&Rectangle::new([(bx, by), (bx + 16, by + 11)], icon_color.filled())).unwrap();
                         
                         // Shackle
                         let sx = bx + 2;
                         let sy = by;
                         let sw = 12; // Shackle width
                         let sh = 6;  // Shackle height (above body)
                         
                         let outline_color = icon_color; //.stroke_width(2); implies using into_shape logic if complicated, but PathElement handles stroke
                         
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

                    }

                    // Interval Control Footer
                    let footer_y = 360;
                    root.draw_text("Update Interval (min):", &font.clone().color(&WHITE), (20, footer_y + 3)).unwrap();
                    
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

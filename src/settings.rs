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
    #[allow(dead_code)]
    context: Context<Rc<Window>>,
    proxy: EventLoopProxy<UserEvent>,
    
    // UI State
    input_text: String,
    is_focused: bool,
    active_charts: Vec<(WindowId, String)>, // Keep track of charts
    cursor_pos: (f64, f64),
}

impl SettingsWindow {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<UserEvent>) -> Self {
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
        }
    }

    pub fn update_active_charts(&mut self, charts: Vec<(WindowId, String)>) {
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

    fn update_active_charts(&mut self, charts: Vec<(WindowId, String)>) {
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
                for (i, (id, _)) in self.active_charts.iter().enumerate() {
                    let btn_y = 130 + (i as i32 * 30);
                    if x >= 230.0 && x <= 290.0 && y >= btn_y as f64 && y <= (btn_y + 25) as f64 {
                        let _ = self.proxy.send_event(UserEvent::DeleteChart(*id));
                    }
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
                    for (i, (_id, symbol)) in self.active_charts.iter().enumerate() {
                         let y = 130 + (i as i32 * 30);
                         root.draw_text(symbol, &font.clone().color(&WHITE), (20, y)).unwrap();
                         
                         // Delete Button
                         let del_hover = self.cursor_pos.0 >= 230.0 && self.cursor_pos.0 <= 290.0 && self.cursor_pos.1 >= y as f64 && self.cursor_pos.1 <= (y + 25) as f64;
                         let del_color = if del_hover { RGBColor(255, 50, 50) } else { RGBColor(200, 0, 0) };
                         root.draw(&Rectangle::new([(230, y), (290, y + 25)], del_color.filled())).unwrap();
                         root.draw_text("Del", &font.clone().color(&WHITE), (245, y + 3)).unwrap();
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

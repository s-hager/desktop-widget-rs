use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};
use window_vibrancy::apply_acrylic;
use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use yahoo_finance_api as yahoo;
use chrono::{DateTime, Utc, TimeZone};
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};
use tray_icon::menu::{Menu, MenuItem, MenuEvent};
use winit::platform::windows::WindowAttributesExtWindows;

#[derive(Debug)]
enum UserEvent {
    DataLoaded(Vec<yahoo::Quote>),
    Error(String),
}

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    context: Option<Context<Rc<Window>>>,
    proxy: EventLoopProxy<UserEvent>,
    quotes: Option<Vec<yahoo::Quote>>,
    tray_icon: Option<TrayIcon>,
    tray_menu: Option<Menu>, // Keep menu alive
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Acrylic Stock Chart")
            .with_transparent(true)
            .with_decorations(true)
            .with_skip_taskbar(true); // Hide from taskbar

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        
        #[cfg(target_os = "windows")]
        if let Err(err) = apply_acrylic(&window, Some((18, 18, 18, 125))) {
             eprintln!("Failed to apply acrylic: {}", err);
        }

        // Initialize Tray Menu
        let tray_menu = Menu::new();
        let quit_i = MenuItem::new("Quit", true, None);
        tray_menu.append(&quit_i).unwrap();

        // Initialize Tray Icon
        let icon_rgba = vec![255u8; 32 * 32 * 4]; // White icon
        let icon = Icon::from_rgba(icon_rgba, 32, 32).unwrap();
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu.clone()))
            .with_icon(icon)
            .with_tooltip("Stock Widget")
            .build()
            .unwrap();
        
        self.tray_icon = Some(tray_icon);
        self.tray_menu = Some(tray_menu);

        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        
        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
             surface.resize(width, height).unwrap();
        }

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);

        // Spawn fetching task
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let provider = yahoo::YahooConnector::new().unwrap();
                match provider.get_quote_range("AAPL", "1d", "1mo").await {
                    Ok(response) => {
                         if let Ok(quotes) = response.quotes() {
                             let _ = proxy.send_event(UserEvent::DataLoaded(quotes));
                         } else {
                             let _ = proxy.send_event(UserEvent::Error("No quotes found".into()));
                         }
                    },
                    Err(e) => {
                        let _ = proxy.send_event(UserEvent::Error(format!("Fetch error: {}", e)));
                    }
                }
            });
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::Resized(size) => {
                 if let Some(surface) = &mut self.surface {
                     if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                         surface.resize(width, height).unwrap();
                         if let Some(window) = &self.window {
                             window.request_redraw();
                         }
                     }
                 }
            },
            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
                    if let Ok(mut buffer) = surface.buffer_mut() {
                        // Clear to transparency (0x00000000)
                        buffer.fill(0);

                        // If we have data, draw the chart
                        if let Some(quotes) = &self.quotes {
                            let width = buffer.width().get();
                            let height = buffer.height().get();
                            
                            // Plotters needs a buffer of u8 (RGB or RGBA) usually, but we have u32 (0RGB).
                            // Helper to wrap buffer? 
                            // Plotters has BitMapBackend. We can transmute slice but endianness matters.
                            // softbuffer is usually 0x00RRGGBB.
                            // Plotters RGB is [R, G, B].
                            
                            // Let's create a temporary Vec<u8> for plotters, draw to it, then copy to softbuffer.
                            // Not efficient but safest for quick impl.
                            // Actually, let's try to map straight to u32 if possible, or use a custom backend.
                            // For now, drawing to a vec<u8> and blitting is reliable.
                            
                            let mut pixel_buffer = vec![0u8; (width * height * 3) as usize];
                            {
                                let root = BitMapBackend::with_buffer(&mut pixel_buffer[..], (width, height)).into_drawing_area();
                                root.fill(&TRANSPARENT).unwrap(); 
                                
                                // Draw Text manually at top-left
                                root.draw_text("AAPL", &("sans-serif", 30).into_font().color(&WHITE), (20, 20)).unwrap();

                                // wait, plotters BitMapBackend doesn't support alpha well usually unless RGBA?
                                // BitMapBackend is RGB usually.
                                // Let's try drawing with a dark background matching acrylic tint?
                                // User said "transparent background" but chart on top.
                                // If we fill with black, it will be black.
                                // If we don't clean it, it's garbage.
                                
                                // Let's use WHITE text/lines.
                                
                                let start_date = DateTime::from_timestamp(quotes.first().unwrap().timestamp as i64, 0).unwrap();
                                let end_date = DateTime::from_timestamp(quotes.last().unwrap().timestamp as i64, 0).unwrap();
                                
                                let min_price = quotes.iter().map(|q| q.low).fold(f64::INFINITY, f64::min);
                                let max_price = quotes.iter().map(|q| q.high).fold(f64::NEG_INFINITY, f64::max);

                            // Label
                                let mut chart = ChartBuilder::on(&root)
                                    .margin(10)
                                    .margin_top(60) // Extra margin for label
                                    // .caption("AAPL", ("sans-serif", 30).into_font().color(&WHITE)) // Removed caption
                                    .set_label_area_size(LabelAreaPosition::Left, 40)
                                    .set_label_area_size(LabelAreaPosition::Bottom, 40)
                                    .build_cartesian_2d(start_date..end_date, min_price..max_price)
                                    .unwrap();

                                chart.configure_mesh()
                                    .axis_style(WHITE)
                                    .bold_line_style(WHITE.mix(0.3))
                                    .light_line_style(WHITE.mix(0.1))
                                    .label_style(("sans-serif", 15).into_font().color(&WHITE))
                                    .draw().unwrap();

                                chart.draw_series(
                                    LineSeries::new(
                                        quotes.iter().map(|q| (
                                            DateTime::from_timestamp(q.timestamp as i64, 0).unwrap(),
                                            q.close
                                        )),
                                        &GREEN,
                                    )
                                ).unwrap();
                            }
                            
                            // Convert RGB buffer to 0RGB u32 buffer
                            for (i, chunk) in pixel_buffer.chunks(3).enumerate() {
                                if i < buffer.len() {
                                    let r = chunk[0] as u32;
                                    let g = chunk[1] as u32;
                                    let b = chunk[2] as u32;
                                    // Simple keying: if 0,0,0 (black), keep transparent?
                                    // No, we cleared to 0. Plotters cleared to what?
                                    // BitMapBackend default fill?
                                    // We called root.fill(&TRANSPARENT)... plotters TRANSPARENT is usually weird in Bitmap.
                                    // Let's assume we want to draw pixels that are NOT background.
                                    // Actually, let's just add the color.
                                    if r != 0 || g != 0 || b != 0 {
                                        buffer[i] = (r << 16) | (g << 8) | b;
                                    }
                                }
                            }
                        }

                        buffer.present().ok();
                    }
                    // window.request_redraw(); // REMOVED to fix high CPU usage
                }
            },
            _ => (),
        }
    }
    
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
         use tray_icon::menu::MenuEvent;
         while let Ok(event) = MenuEvent::receiver().try_recv() {
             // We only have one item "Quit", so exit on any event for now
             if event.id.0 == "2" { // Hack: check ID? Or just exit. 
                 // Wait, MenuItem::new returns item with auto ID?
                 // Let's just assume it's quit for now, or match text.
                 // Actually, best to store ID or just exit since it's the only item.
                 event_loop.exit();
             }
             // Actually, `tray-icon` docs say IDs are auto-generated if constructed simply.
             // But we can check since we only have Quit.
             event_loop.exit();
         }
    }


    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::DataLoaded(new_quotes) => {
                println!("Loaded {} quotes", new_quotes.len());
                self.quotes = Some(new_quotes);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UserEvent::Error(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let proxy = event_loop.create_proxy();
    let mut app = App { 
        window: None, 
        surface: None, 
        context: None, 
        proxy,
        quotes: None,
        tray_icon: None,
        tray_menu: None
    };
    
    event_loop.run_app(&mut app).unwrap();
}

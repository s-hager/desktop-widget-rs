use winit::window::Window;
use winit::window::WindowId;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::event::WindowEvent;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use crate::common::{WindowHandler, UserEvent};
use chrono::DateTime;
use winit::platform::windows::WindowAttributesExtWindows;
use window_vibrancy::apply_acrylic;
use yahoo_finance_api as yahoo;

pub struct ChartWindow {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    // context is held by surface usually, but we might need to keep it if we recreate surface? 
    // Softbuffer 0.4: Surface::new(&context, window). 
    // Context needs to be kept alive? Yes.
    #[allow(dead_code)]
    context: Context<Rc<Window>>, 
    symbol: String,
    currency: String,
    quotes: Option<Vec<yahoo::Quote>>,
}

impl ChartWindow {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<UserEvent>, symbol: String) -> Self {
        let window_attributes = Window::default_attributes()
            .with_title(&format!("Stock Chart - {}", symbol))
            .with_transparent(true)
            .with_decorations(true)
            .with_skip_taskbar(true); 

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(target_os = "windows")]
        if let Err(err) = apply_acrylic(&window, Some((18, 18, 18, 125))) {
             eprintln!("Failed to apply acrylic: {}", err);
        }

        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        
        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
             surface.resize(width, height).unwrap();
        }

        // Spawn fetching task
        let proxy = proxy.clone();
        let symbol_clone = symbol.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let provider = yahoo::YahooConnector::new().unwrap();
                match provider.get_quote_range(&symbol_clone, "1d", "1mo").await {
                    Ok(response) => {
                         let currency = response.metadata().ok().and_then(|m| m.currency.clone()).unwrap_or("USD".to_string());
                         if let Ok(quotes) = response.quotes() {
                             let _ = proxy.send_event(UserEvent::DataLoaded(symbol_clone, quotes, currency));
                         } else {
                             let _ = proxy.send_event(UserEvent::Error(symbol_clone, "No quotes found".into()));
                         }
                    },
                    Err(e) => {
                        let _ = proxy.send_event(UserEvent::Error(symbol_clone, format!("Fetch error: {}", e)));
                    }
                }
            });
        });

        Self {
            window,
            surface,
            context,
            symbol,
            currency: "USD".to_string(),
            quotes: None,
        }
    }
}

impl WindowHandler for ChartWindow {
    fn window_id(&self) -> WindowId {
        self.window.id()
    }

    fn handle_event(&mut self, event: WindowEvent, _event_loop: &ActiveEventLoop) {
         match event {
            WindowEvent::CloseRequested => {
                // Main loop handles destruction, but maybe we should trigger it here?
                // For now, assume main loop removes us from map.
            },
            WindowEvent::Resized(size) => {
                self.resize(size);
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

    fn update_data(&mut self, quotes: Vec<yahoo::Quote>, currency: String) {
        self.quotes = Some(quotes);
        self.currency = currency;
        self.window.request_redraw();
    }

    fn redraw(&mut self) {
        if let Ok(mut buffer) = self.surface.buffer_mut() {
            buffer.fill(0);

            if let Some(quotes) = &self.quotes {
                let width = buffer.width().get();
                let height = buffer.height().get();
                
                let first_quote = quotes.first().unwrap();
                let last_quote = quotes.last().unwrap();
                let first_price = first_quote.close;
                let last_price = last_quote.close;
                let diff = last_price - first_price;
                let percent_change = (diff / first_price) * 100.0;
                
                let color = if diff >= 0.0 { &GREEN } else { &RED };
                let sign = if diff >= 0.0 { "+" } else { "" };

                let symbol_txt = match self.currency.as_str() {
                    "USD" => "$",
                    "EUR" => "€",
                    "GBP" => "£",
                    "JPY" => "¥",
                    _ => &self.currency,
                };

                let mut pixel_buffer = vec![0u8; (width * height * 3) as usize];
                {
                    let root = BitMapBackend::with_buffer(&mut pixel_buffer[..], (width, height)).into_drawing_area();
                    root.fill(&TRANSPARENT).unwrap(); 
                    
                    let font = ("sans-serif", 30).into_font();
                    let padding = 20;
                    let mut current_x = 20;

                    // Symbol
                    root.draw_text(&self.symbol, &font.clone().color(&WHITE), (current_x, 20)).unwrap();
                    let (w, _) = font.box_size(&self.symbol).unwrap();
                    current_x += w as i32 + padding;
                    
                    // Price
                    let price_text = format!("{}{:.2}", symbol_txt, last_price);
                    root.draw_text(&price_text, &font.clone().color(&WHITE), (current_x, 20)).unwrap();
                    let (w, _) = font.box_size(&price_text).unwrap();
                    current_x += w as i32 + padding;

                    // Change
                    let change_text = format!("{}{:.2} ({}{:.2}%)", sign, diff, sign, percent_change);
                    root.draw_text(&change_text, &font.clone().color(color), (current_x, 20)).unwrap();
                    let (w, _) = font.box_size(&change_text).unwrap();
                    current_x += w as i32 + padding;

                    // Update Window Min Size
                    let min_width = current_x as u32;
                    let min_height = 300; 
                    self.window.set_min_inner_size(Some(winit::dpi::LogicalSize::new(min_width as f64, min_height as f64)));
                    
                    // Chart
                    let start_date = DateTime::from_timestamp(quotes.first().unwrap().timestamp as i64, 0).unwrap();
                    let end_date = DateTime::from_timestamp(quotes.last().unwrap().timestamp as i64, 0).unwrap();
                    
                    let min_price = quotes.iter().map(|q| q.low).fold(f64::INFINITY, f64::min);
                    let max_price = quotes.iter().map(|q| q.high).fold(f64::NEG_INFINITY, f64::max);
                    
                    let range = max_price - min_price;
                    let use_decimals = range < 1.0 || max_price < 2.0;
                    
                    let x_labels = (width / 120).max(2) as usize;
                    let y_labels = (height / 60).max(2) as usize;

                    let mut chart = ChartBuilder::on(&root)
                        .margin(10)
                        .margin_top(60) 
                        .set_label_area_size(LabelAreaPosition::Left, 40)
                        .set_label_area_size(LabelAreaPosition::Bottom, 40)
                        .build_cartesian_2d(start_date..end_date, min_price..max_price)
                        .unwrap();

                    chart.configure_mesh()
                        .axis_style(WHITE)
                        .bold_line_style(WHITE.mix(0.3))
                        .light_line_style(TRANSPARENT)
                        .label_style(("sans-serif", 15).into_font().color(&WHITE))
                        .x_labels(x_labels)
                        .y_labels(y_labels)
                        .x_label_formatter(&|d| d.format("%b %e").to_string())
                        .y_label_formatter(&|y| {
                            if use_decimals {
                                format!("{:.2}", y)
                            } else {
                                format!("{:.0}", y)
                            }
                        })
                        .draw().unwrap();

                    chart.draw_series(
                        LineSeries::new(
                            quotes.iter().map(|q| (
                                DateTime::from_timestamp(q.timestamp as i64, 0).unwrap(),
                                q.close
                            )),
                            color,
                        )
                    ).unwrap();
                }
                
                for (i, chunk) in pixel_buffer.chunks(3).enumerate() {
                    if i < buffer.len() {
                        let r = chunk[0] as u32;
                        let g = chunk[1] as u32;
                        let b = chunk[2] as u32;
                        if r != 0 || g != 0 || b != 0 {
                            buffer[i] = (r << 16) | (g << 8) | b;
                        }
                    }
                }
            }
            buffer.present().ok();
        }
    }
}

use winit::window::{Window, WindowLevel};
use winit::window::WindowId;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::event::{WindowEvent, ElementState, MouseButton};
use std::rc::Rc;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use crate::common::{WindowHandler, UserEvent};
use chrono::{DateTime, Local, Utc};
use time::OffsetDateTime;
use std::collections::HashMap;
use std::time::Instant;
use winit::platform::windows::WindowAttributesExtWindows;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use yahoo_finance_api as yahoo;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use crate::language::{Language, AppError};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::MARGINS;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, WPARAM, LPARAM, LRESULT, RECT, FARPROC, BOOL};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{SetWindowSubclass, DefSubclassProc};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, SetWindowPos, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::{LoadLibraryA, GetProcAddress};

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct ACCENT_POLICY {
    AccentState: u32,
    AccentFlags: u32,
    GradientColor: u32,
    AnimationId: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct WINDOWCOMPOSITIONATTRIBDATA {
    Attrib: u32,
    pvData: *mut c_void,
    cbData: usize,
}

#[cfg(target_os = "windows")]
#[derive(PartialEq)]
#[repr(C)]
#[allow(non_camel_case_types)]
// #[allow(dead_code)]
enum ACCENT_STATE {
    // ACCENT_DISABLED = 0,
    // ACCENT_ENABLE_GRADIENT = 1,
    // ACCENT_ENABLE_TRANSPARENTGRADIENT = 2,
    // ACCENT_ENABLE_BLURBEHIND = 3,
    ACCENT_ENABLE_ACRYLICBLURBEHIND = 4,
    // ACCENT_INVALID_STATE = 5,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uid_subclass: usize,
    _dw_ref_data: usize,
) -> LRESULT {
    const WM_NCHITTEST: u32 = 0x0084;
    const WM_MOUSEACTIVATE: u32 = 0x0021;
    const MA_NOACTIVATE: LRESULT = 3;

    // Prevent activation on click
    if msg == WM_MOUSEACTIVATE {
        return MA_NOACTIVATE;
    }

    // Only handle Resize if Unlocked (ref_data == 0)
    let locked = _dw_ref_data == 1;

    if msg == WM_NCHITTEST && !locked {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: GetWindowRect is called with a valid HWND and pointer to RECT.
        unsafe { GetWindowRect(hwnd, &mut rect) };
        
        let x = (lparam & 0xFFFF) as i16 as i32;
        let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
        
        let border_width = 8;
        let bottom_border_height = 8; 

        let left = x < rect.left + border_width;
        let right = x >= rect.right - border_width;
        let top = y < rect.top + border_width;
        let bottom = y >= rect.bottom - bottom_border_height;

        if top && left { return HTTOPLEFT as LRESULT; }
        if top && right { return HTTOPRIGHT as LRESULT; }
        if bottom && left { return HTBOTTOMLEFT as LRESULT; }
        if bottom && right { return HTBOTTOMRIGHT as LRESULT; }
        if left { return HTLEFT as LRESULT; }
        if right { return HTRIGHT as LRESULT; }
        if top { return HTTOP as LRESULT; }
        if bottom { return HTBOTTOM as LRESULT; }
    }

    // SAFETY: DefSubclassProc is safe to call with valid HWND.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(target_os = "windows")]
fn get_function_impl(library: &str, function: &str) -> Option<FARPROC> {
    let module = unsafe { LoadLibraryA(library.as_ptr()) };
    if module == 0 {
        return None;
    }
    Some(unsafe { GetProcAddress(module, function.as_ptr()) })
}

#[cfg(target_os = "windows")]
unsafe fn set_window_composition_attribute(hwnd: HWND, accent_state: ACCENT_STATE, color: Option<(u8, u8, u8, u8)>) {
    type SetWindowCompositionAttributeFn = unsafe extern "system" fn(HWND, *mut WINDOWCOMPOSITIONATTRIBDATA) -> BOOL;

    let library = "user32.dll\0";
    let function = "SetWindowCompositionAttribute\0";

    if let Some(proc) = get_function_impl(library, function) {
        // SAFETY: Casting FARPROC to function pointer signature we expect.
        let set_window_composition_attribute: SetWindowCompositionAttributeFn = unsafe { std::mem::transmute(proc) };
        
        let mut color = color.unwrap_or((0, 0, 0, 0));
        let is_acrylic = accent_state == ACCENT_STATE::ACCENT_ENABLE_ACRYLICBLURBEHIND;
        if is_acrylic && color.3 == 0 {
             color.3 = 1;
        }

        let mut policy = ACCENT_POLICY {
            AccentState: accent_state as u32,
            AccentFlags: if is_acrylic { 0 } else { 2 },
            GradientColor: (color.0 as u32)
                | ((color.1 as u32) << 8)
                | ((color.2 as u32) << 16)
                | ((color.3 as u32) << 24),
            AnimationId: 0,
        };

        let mut data = WINDOWCOMPOSITIONATTRIBDATA {
            Attrib: 0x13, // WCA_ACCENT_POLICY
            pvData: &mut policy as *mut _ as *mut c_void,
            cbData: std::mem::size_of_val(&policy),
        };

        // SAFETY: Calling loaded function pointer.
        unsafe { set_window_composition_attribute(hwnd, &mut data) };
    }
}

fn apply_shadow(window: &Window) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                let hwnd = handle.hwnd.get() as HWND;
                let margins = MARGINS {
                    cxLeftWidth: 1,
                    cxRightWidth: 1,
                    cyTopHeight: 1,
                    cyBottomHeight: 1,
                };
                unsafe {
                    DwmExtendFrameIntoClientArea(hwnd, &margins);
                    // Use a separate function or logic to manage subclass based on lock state
                    // SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0); // Moved to set_locked logic
                    
                    // Manual Acrylic Application
                    set_window_composition_attribute(
                        hwnd, 
                        ACCENT_STATE::ACCENT_ENABLE_ACRYLICBLURBEHIND, 
                        Some((18, 18, 18, 125))
                    );
                }
            }
        }
    }
}

fn update_subclass(window: &Window, locked: bool) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                let hwnd = handle.hwnd.get() as HWND;
                let ref_data = if locked { 1 } else { 0 };
                unsafe {
                     SetWindowSubclass(hwnd, Some(subclass_proc), 1, ref_data);
                }
            }
        }
    }
}

pub struct ChartWindow {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    // context is held by surface usually, but we might need to keep it if we recreate surface? 
    // Softbuffer 0.4: Surface::new(&context, window). 
    // Context needs to be kept alive? Yes.
    _context: Context<Rc<Window>>, 
    symbol: String,
    currency: String,
    quotes: Option<Vec<yahoo::Quote>>,
    locked: bool,
    proxy: EventLoopProxy<UserEvent>,
    last_fetch_time: Option<DateTime<Local>>,
    timeframe: String,
    
    // Cache: Timeframe -> (Quotes, Currency, FetchTime)
    cache: HashMap<String, (Vec<yahoo::Quote>, String, DateTime<Local>)>,
    pending_timeframe: Option<String>,
    last_timeframe_change: Option<Instant>,
    language: Language,
}

use crate::config::ChartConfig;

impl ChartWindow {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<UserEvent>, symbol: String, config: Option<ChartConfig>, language: Language) -> Self {
        // ... (attributes setup)
        let mut window_attributes = Window::default_attributes()
            .with_title(&format!("Stock Chart - {}", symbol))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnBottom)
            .with_skip_taskbar(true)
            .with_visible(false); 

        if let Some(cfg) = &config {
            window_attributes = window_attributes
                .with_position(winit::dpi::PhysicalPosition::new(cfg.x, cfg.y))
                .with_inner_size(winit::dpi::PhysicalSize::new(cfg.width, cfg.height));
        }

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(target_os = "macos")]
        apply_vibrancy(&*window, NSVisualEffectMaterial::HudWindow, None, None).expect("Unsupported platform!");

        // Since the window-shadows crate is deprecated and incompatible with the version of winit used in this project (causing the build failures you saw), I implemented the shadow logic manually using the windows-sys crate.
        apply_shadow(&window);

        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        
        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
             surface.resize(width, height).unwrap();
        }


        
        let mut chart = Self {
            window,
            surface,
            _context: context,
            symbol: symbol.clone(),
            currency: "USD".to_string(),
            quotes: None,
            locked: true,
            proxy,
            last_fetch_time: None,
            timeframe: config.as_ref().and_then(|c| c.timeframe.clone()).unwrap_or("1M".to_string()),
            cache: HashMap::new(),
            pending_timeframe: None,
            last_timeframe_change: None,
            language,
        };
        
        // Initialize subclass
        update_subclass(&chart.window, true);
        
        // Initial Fetch
        chart.refresh();

        chart
    }

    fn load_from_cache(&mut self) {
         if let Some((quotes, currency, ts)) = self.cache.get(&self.timeframe) {
             self.quotes = Some(quotes.clone());
             self.currency = currency.clone();
             self.last_fetch_time = Some(*ts);
             self.window.request_redraw();
         }
    }

    fn fetch_data(&self) {
        let proxy = self.proxy.clone();
        let symbol = self.symbol.clone();
        
        let timeframe = self.timeframe.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let provider = yahoo::YahooConnector::new().unwrap();
                
                if timeframe == "1W" {
                    // Stitching 7 days of 5m data
                    let mut all_quotes = Vec::new();
                    let now = Utc::now();
                    let start = now - chrono::Duration::days(7);
                    
                    let mut currency = "USD".to_string();       
                    // Fetch day by day to allow high resolution "5m"
                    for i in 0..8 {
                         let chunk_start = start + chrono::Duration::days(i as i64);
                         let chunk_end = chunk_start + chrono::Duration::days(1);
                         
                         if chunk_start > now { break; }

                         let sys_start: std::time::SystemTime = chunk_start.into();
                         let sys_end: std::time::SystemTime = chunk_end.into();
                         let odt_start = OffsetDateTime::from(sys_start);
                         let odt_end = OffsetDateTime::from(sys_end);

                         if let Ok(response) = provider.get_quote_history_interval(&symbol, odt_start, odt_end, "5m").await {
                             if let Ok(meta) = response.metadata() {
                                 currency = meta.currency.clone().unwrap_or("USD".to_string());
                             }
                             if let Ok(quotes) = response.quotes() {
                                 all_quotes.extend(quotes);
                             }
                         }
                    }
                    
                    // Dedup and sort
                    all_quotes.sort_by_key(|q| q.timestamp);
                    all_quotes.dedup_by_key(|q| q.timestamp);
                    
                     if !all_quotes.is_empty() {
                          let _ = proxy.send_event(UserEvent::DataLoaded(symbol, all_quotes, currency));
                     } else {
                          let _ = proxy.send_event(UserEvent::Error(symbol, AppError::WeekDataError));
                     }

                } else {
                    // Standard fetching
                    let (interval, range) = match timeframe.as_str() {
                        "1D" => ("2m", "1d"),
                        "1M" => ("1d", "1mo"),
                        "3M" => ("1d", "3mo"),
                        "6M" => ("1d", "6mo"),
                        "1Y" => ("1d", "1y"),
                        "YTD" => ("1d", "ytd"),
                        _ => ("1d", "1mo"),
                    };
                    match provider.get_quote_range(&symbol, interval, range).await {
                        Ok(response) => {
                             let currency = response.metadata().ok().and_then(|m| m.currency.clone()).unwrap_or("USD".to_string());
                              if let Ok(quotes) = response.quotes() {
                                  let _ = proxy.send_event(UserEvent::DataLoaded(symbol, quotes, currency));
                              } else {
                                  let _ = proxy.send_event(UserEvent::Error(symbol, AppError::NoQuotesFound));
                              }
                         },
                         Err(e) => {
                             let _ = proxy.send_event(UserEvent::Error(symbol, AppError::FetchError(e.to_string())));
                         }
                    }
                }
            });
        });
    }

    fn force_to_bottom(&self) {
        #[cfg(target_os = "windows")]
        {
            if let Ok(handle) = self.window.window_handle() {
                if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                    let hwnd = handle.hwnd.get() as HWND;
                    unsafe {
                        SetWindowPos(hwnd, 1 as HWND, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    }
                }
            }
        }
    }
}

impl WindowHandler for ChartWindow {
    fn window_id(&self) -> WindowId {
        self.window.id()
    }


    fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
        update_subclass(&self.window, locked);
        self.window.request_redraw();
    }

    fn set_language(&mut self, language: Language) {
        self.language = language;
        self.window.request_redraw();
    }

    fn refresh(&mut self) {
        // If data is older than 30 mins, fetch new
        if let Some(last) = self.last_fetch_time {
             if (Local::now() - last).num_minutes() >= 30 {
                 self.fetch_data();
             }
        } else {
             self.fetch_data();
        }
    }

    fn set_timeframe(&mut self, timeframe: String) {
        // Check cache first - if valid, apply immediately (no debounce needed)
        let mut cache_hit = false;
        if let Some((_, _, ts)) = self.cache.get(&timeframe) {
            // Check if outdated (30 mins)
            if (Local::now() - *ts).num_minutes() < 30 {
                cache_hit = true;
            }
        }

        if cache_hit {
            // Apply immediate
            self.timeframe = timeframe;
            self.pending_timeframe = None;
            self.load_from_cache();
        } else {
            self.pending_timeframe = Some(timeframe.clone());
            self.last_timeframe_change = Some(Instant::now());
        }
    }

    fn tick(&mut self) {
        if let Some(pending) = &self.pending_timeframe {
            if let Some(last_change) = self.last_timeframe_change {
                 if last_change.elapsed().as_millis() > 500 {
                     // Commit
                     self.timeframe = pending.clone();
                     self.pending_timeframe = None;
                     self.fetch_data();
                 }
            }
        }
    }

    fn has_data(&self) -> bool {
        self.last_fetch_time.is_some()
    }
    
    fn get_config(&self) -> Option<ChartConfig> {
        let size = self.window.inner_size();
        let pos = self.window.outer_position().unwrap_or(winit::dpi::PhysicalPosition::new(0, 0));
        Some(ChartConfig {
            symbol: self.symbol.clone(),
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
            timeframe: self.pending_timeframe.clone().or_else(|| Some(self.timeframe.clone())),
        })
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
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if !self.locked {
                    let _ = self.window.drag_window();
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

    fn update_data(&mut self, quotes: Vec<yahoo::Quote>, currency: String) {
        self.quotes = Some(quotes.clone());
        self.currency = currency.clone();
        let now = Local::now();
        self.last_fetch_time = Some(now);
        
        // Update Cache
        self.cache.insert(self.timeframe.clone(), (quotes, currency, now));

        self.window.set_visible(true);
        self.force_to_bottom();
        self.window.request_redraw();
    }

    fn redraw(&mut self) {
        if let Ok(mut buffer) = self.surface.buffer_mut() {
            buffer.fill(0);

            let width = buffer.width().get();
            let height = buffer.height().get();

            // (Border drawing moved to end)

            if let Some(quotes) = &self.quotes {
                // let width = buffer.width().get(); // Already defined
                // let height = buffer.height().get();
                
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
                        .x_label_formatter(&|d| {
                            let date = DateTime::from_timestamp(d.timestamp(), 0).unwrap().with_timezone(&Local);
                            let lang = self.language;

                            if self.timeframe == "1D" {
                                crate::language::format_time(date)
                            } else if self.timeframe == "1W" {
                                let duration = end_date.signed_duration_since(start_date);
                                if duration.num_days() <= 2 {
                                    crate::language::format_weekday_time(lang, date)
                                } else {
                                    crate::language::format_month_day(lang, date)
                                }
                            } else {
                                crate::language::format_month_day(lang, date)
                            }
                        })
                        .y_label_formatter(&|y| {
                            if use_decimals {
                                format!("{:.2}", y)
                            } else {
                                format!("{:.0}", y)
                            }
                        })
                        .draw().unwrap();

                    chart.draw_series(
                        AreaSeries::new(
                            quotes.iter().map(|q| (
                                DateTime::from_timestamp(q.timestamp as i64, 0).unwrap(),
                                q.close
                            )),
                            min_price,
                            color.mix(0.15).filled(),
                        )
                    ).unwrap();

                    chart.draw_series(
                        LineSeries::new(
                            quotes.iter().map(|q| (
                                DateTime::from_timestamp(q.timestamp as i64, 0).unwrap(),
                                q.close
                            )),
                            color,
                        )
                    ).unwrap();
                    
                    // Draw Timestamp
                    if let Some(ts) = self.last_fetch_time {
                        let time_str = format!("{}", ts.format("%Y-%m-%d %H:%M:%S"));
                        let ts_font = ("sans-serif", 14).into_font();
                        let (tw, th) = ts_font.box_size(&time_str).unwrap();
                        // Bottom Right
                        let tx = (width as i32) - (tw as i32) - 10;
                        let ty = (height as i32) - (th as i32) - 5;
                        root.draw_text(&time_str, &ts_font.color(&WHITE.mix(0.5)), (tx, ty)).unwrap();
                    }
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
            
            // Draw yellow frame if unlocked
            if !self.locked {
                let width = buffer.width().get() as i32;
                let height = buffer.height().get() as i32;
                let frame_color = 0xFFFF00; // Yellow
                let thickness = 3; 
                let radius = 12; // Radius for rounded corners

                for y in 0..height {
                    for x in 0..width {
                        let mut in_border = false;

                        // Check corners
                        if x < radius && y < radius { // Top-Left
                            let d = ((x - radius).pow(2) + (y - radius).pow(2)) as f64;
                            let r_out = (radius as f64).powi(2);
                            let r_in = ((radius - thickness) as f64).powi(2);
                            if d <= r_out && d >= r_in { in_border = true; }
                        } else if x >= width - radius && y < radius { // Top-Right
                            let d = ((x - (width - radius)).pow(2) + (y - radius).pow(2)) as f64;
                            let r_out = (radius as f64).powi(2);
                            let r_in = ((radius - thickness) as f64).powi(2);
                            if d <= r_out && d >= r_in { in_border = true; }
                        } else if x < radius && y >= height - radius { // Bottom-Left
                             let d = ((x - radius).pow(2) + (y - (height - radius)).pow(2)) as f64;
                             let r_out = (radius as f64).powi(2);
                             let r_in = ((radius - thickness) as f64).powi(2);
                             if d <= r_out && d >= r_in { in_border = true; }
                        } else if x >= width - radius && y >= height - radius { // Bottom-Right
                             let d = ((x - (width - radius)).pow(2) + (y - (height - radius)).pow(2)) as f64;
                             let r_out = (radius as f64).powi(2);
                             let r_in = ((radius - thickness) as f64).powi(2);
                             if d <= r_out && d >= r_in { in_border = true; }
                        } else {
                            // Straight Edges
                            // Top Edge (between rounded corners)
                            if y < thickness && x >= radius && x < width - radius { in_border = true; }
                            // Bottom Edge
                            if y >= height - thickness && x >= radius && x < width - radius { in_border = true; }
                            // Left Edge
                            if x < thickness && y >= radius && y < height - radius { in_border = true; }
                            // Right Edge
                            if x >= width - thickness && y >= radius && y < height - radius { in_border = true; }
                        }

                        if in_border {
                             let idx = (y * width + x) as usize;
                             if idx < buffer.len() {
                                 buffer[idx] = frame_color;
                             }
                        }
                    }
                }
            }

            buffer.present().ok();
        }
    }
}

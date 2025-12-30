use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use window_vibrancy::apply_acrylic;
use std::num::NonZeroU32;
use softbuffer::{Context, Surface};

use std::rc::Rc; // Import Rc


struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    context: Option<Context<Rc<Window>>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Acrylic Window")
            .with_transparent(true)
            .with_decorations(true);

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        
        #[cfg(target_os = "windows")]
        if let Err(err) = apply_acrylic(&window, Some((18, 18, 18, 125))) {
             eprintln!("Failed to apply acrylic: {}", err);
        }

        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        
        // Resize surface initially
        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
             surface.resize(width, height).unwrap();
        }

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
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
                     }
                 }
            },
            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
                    if let Ok(mut buffer) = surface.buffer_mut() {
                         buffer.fill(0);
                         buffer.present().ok();
                    }
                    window.request_redraw(); 
                }
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let mut app = App { window: None, surface: None, context: None };
    event_loop.run_app(&mut app).unwrap();
}

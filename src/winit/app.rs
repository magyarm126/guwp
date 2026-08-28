use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::WindowId,
};
use winit::window::Window;

pub struct App {
    window: Option<Arc<Window>>,
}

impl Default for App {
    fn default() -> Self {
        return App { window: None };
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("GUWP")
                        .with_inner_size(
                            winit::dpi::PhysicalSize::new(800, 600),
                        ),
                )
                .unwrap(),
        );
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {event_loop.exit();}
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
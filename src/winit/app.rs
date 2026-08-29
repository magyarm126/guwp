use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::WindowId,
};
use winit::event_loop::EventLoop;
use winit::window::Window;
use crate::wgpu::state::State;

pub struct App {
    state: Option<State>,
}

impl App {
    pub(crate) fn from_event_loop(event_loop: &EventLoop<()>) -> Self {
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
        let state = pollster::block_on(State::new(window));
        App {
            state: Some(state),
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state.as_ref().unwrap().window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {event_loop.exit();}
            WindowEvent::RedrawRequested => {
                self.state.as_mut().unwrap().render();
            }
            WindowEvent::Resized(physical_size) => self.state.as_mut().unwrap().resize(physical_size),
            _ => {}
        }
    }
}
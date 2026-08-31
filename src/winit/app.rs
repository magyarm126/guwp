use crate::winit::state::State;
use std::sync::Arc;
use winit::event_loop::EventLoop;
use winit::window::Window;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::WindowId,
};

#[derive(Default)]
pub struct App {
    state: Option<State>,
}

impl App {
    fn from_event_loop(event_loop: &ActiveEventLoop) -> State {
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
        state
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(Self::from_event_loop(event_loop));
        self.state.as_mut().unwrap().redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {event_loop.exit();}
            WindowEvent::RedrawRequested => {
                self.state.as_mut().unwrap().update();
                self.state.as_mut().unwrap().render();
                self.state.as_mut().unwrap().redraw();
            }
            WindowEvent::Resized(physical_size) => self.state.as_mut().unwrap().resize(physical_size),
            _ => {}
        }
    }
}
use crate::winit::state::State;
use std::sync::Arc;
use std::time::Instant;
use winit::window::{Window, WindowAttributes};
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
    fn from_event_loop(event_loop: &dyn ActiveEventLoop) -> State {
        let window: Arc<dyn Window> = Arc::from(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("GUWP")
                )
                .unwrap(),
        );
        let state = pollster::block_on(State::new(window));
        state
    }
}
impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.state = Some(Self::from_event_loop(event_loop));
        self.state.as_mut().unwrap().redraw();
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {event_loop.exit();}
            WindowEvent::RedrawRequested => {

                let state = self.state.as_mut().unwrap();

                state.loopcounter += 1;
                let start = Instant::now();

                state.update();

                let after_update = Instant::now();


                let mut after_render;

                if state.render() {
                    after_render = Instant::now();
                    state.redraw();
                }

                /*
                println!(
                    "update: {:.3} ms | render: {:.3} ms | total: {:.3} ms",
                    (after_update - start).as_secs_f64() * 1000.0,
                    (after_render - after_update).as_secs_f64() * 1000.0,
                    (after_render - start).as_secs_f64() * 1000.0,
                );
                 */
            }
            WindowEvent::SurfaceResized(physical_size) => {
                state.resize(physical_size);
                state.redraw();
            },
            _ => {
                //println!("Unhandled event: {:?}", event);
            }
        }
    }
}
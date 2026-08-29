use ::winit::event_loop::{ControlFlow, EventLoop};
use crate::winit::app::App;

mod winit;
mod wgpu;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::from_event_loop(&event_loop);

    //std::process::exit(126);

    event_loop.run_app(&mut app).unwrap();

}

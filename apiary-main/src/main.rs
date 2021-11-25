use log::LevelFilter;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use apiary::app::ApiaryApp;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format()
        .filter_level(LevelFilter::Info)
        .init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Apiary Demo")
        .build(&event_loop)
        .unwrap();

    let mut app = ApiaryApp::init(&window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        //let mut a = &mut app;

        match event {
            Event::MainEventsCleared => {
                *control_flow = app.update(&window).unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                app.shutdown().unwrap();
                *control_flow = ControlFlow::Exit
            }
            _ => (),
        }
    });
}

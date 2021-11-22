use log::LevelFilter;
//use rafx_api::{RafxApi, RafxError};
//use rafx_api::raw_window_handle::HasRawWindowHandle;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use apiary::app::ApiaryApp;

/*
impl std::error::Error for ApiaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            ApiaryError::RafxError(e) => Some(e),
        }
    }
}
*/


fn main() {
    println!("Hello, world!");

    env_logger::Builder::from_default_env()
        .default_format()
        //.default_format_timestamp_nanos(true)
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
                //window.request_redraw();
                *control_flow = app.update(&window).unwrap();
            }
//            Event::RedrawRequested(_) => {
            //              *control_flow = app.update(&window).unwrap();
            //        }
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

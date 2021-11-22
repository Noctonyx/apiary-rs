use rafx_api::RafxApi;
use rafx_api::raw_window_handle::HasRawWindowHandle;
use winit::{
    event_loop::{ControlFlow}
};
use winit::window::Window;
use crate::error::ApiaryResult;

pub struct ApiaryApp {
    api: RafxApi,
}

impl ApiaryApp {
    pub fn update(&mut self, _window: &Window) -> ApiaryResult<winit::event_loop::ControlFlow> {
        profiling::scope!("Main Loop");
        profiling::finish_frame!();
        Ok(ControlFlow::Poll)
    }

    pub fn init(window: &dyn HasRawWindowHandle) -> ApiaryResult<Self> {
        profiling::register_thread!("Main Thread");
        //#[cfg(feature = "profile-with-optick")]
        //  profiling::optick::register_thread("Main Thread");

        //#[cfg(feature = "profile-with-tracy")]
        //profiling::tracy_client::set_thread_name("Main Thread");

        let api = unsafe { RafxApi::new(window, &Default::default())? };
        {}
        Ok(ApiaryApp { api })
    }

    pub fn shutdown(&mut self) -> ApiaryResult<()> {
        self.api.destroy()?;
        Ok(())
    }
}

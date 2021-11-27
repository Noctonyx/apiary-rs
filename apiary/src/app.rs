use crate::error::ApiaryResult;
use crate::rendering::rendering_init;
use crate::scene::SceneManager;
use crate::time::TimeState;
use legion::{Resources, World};
use rafx_api::{RafxApi, RafxExtents2D};
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::AssetManager;
use rafx_plugins::assets::font::FontAsset;
use rafx_plugins::features::egui::WinitEguiManager;
use rafx_renderer::daemon::AssetDaemonOpt;
use rafx_renderer::{AssetSource, ViewportsResource};
use std::net::{AddrParseError, SocketAddr};
use std::path::PathBuf;
use std::time::Instant;
use structopt::StructOpt;
use winit::event_loop::ControlFlow;

pub struct ApiaryApp {
    api: RafxApi,

    resources: Resources,
    world: World,
    scene_manager: SceneManager,
}

#[derive(StructOpt)]
pub struct ApiaryArgs {
    /// Path to the packfile
    #[structopt(name = "packfile", long, parse(from_os_str))]
    pub packfile: Option<std::path::PathBuf>,

    #[structopt(skip)]
    pub packbuffer: Option<&'static [u8]>,

    #[structopt(name = "external-daemon", long)]
    pub external_daemon: bool,

    #[structopt(flatten)]
    pub daemon_args: AssetDaemonArgs,
}

impl ApiaryArgs {
    fn asset_source(&self) -> Option<AssetSource> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(packfile) = &self.packfile {
            return Some(AssetSource::Packfile(packfile.to_path_buf()));
        }

        {
            return Some(AssetSource::Daemon {
                external_daemon: self.external_daemon,
                daemon_args: self.daemon_args.clone().into(),
            });
        }
    }
}

#[derive(StructOpt, Debug, Clone)]
pub struct AssetDaemonArgs {
    /// Path to the asset metadata database directory.
    #[structopt(name = "db", long, parse(from_os_str), default_value = ".assets_db")]
    pub db_dir: PathBuf,
    /// Socket address for the daemon to listen for connections, e.g. "127.0.0.1:9999".
    #[structopt(short, long, parse(try_from_str = parse_socket_addr), default_value = "127.0.0.1:9999")]
    pub address: SocketAddr,
    /// Directories to watch for assets.
    #[structopt(parse(from_os_str), default_value = "assets")]
    pub asset_dirs: Vec<PathBuf>,
}

impl Into<AssetDaemonOpt> for AssetDaemonArgs {
    fn into(self) -> AssetDaemonOpt {
        AssetDaemonOpt {
            db_dir: self.db_dir,
            address: self.address,
            asset_dirs: self.asset_dirs,
        }
    }
}

/// Parses a string as a socket address.
fn parse_socket_addr(s: &str) -> std::result::Result<SocketAddr, AddrParseError> {
    s.parse()
}

impl ApiaryApp {
    //#[profiling::function]
    pub fn update(
        &mut self,
        window: &winit::window::Window,
    ) -> ApiaryResult<winit::event_loop::ControlFlow> {
        profiling::scope!("Main Loop");

        let t0 = Instant::now();

        //
        // Update time
        //
        {
            self.resources.get_mut::<TimeState>().unwrap().update();
        }

        {
            let mut viewports_resource = self.resources.get_mut::<ViewportsResource>().unwrap();
            let physical_size = window.inner_size();
            viewports_resource.main_window_size = RafxExtents2D {
                width: physical_size.width,
                height: physical_size.height,
            };
        }

        //
        // Update assets
        //
        {
            profiling::scope!("update asset resource");
            let mut asset_resource = self.resources.get_mut::<AssetResource>().unwrap();
            asset_resource.update();
        }

        //
        // Update graphics resources
        //
        {
            profiling::scope!("update asset loaders");
            let mut asset_manager = self.resources.get_mut::<AssetManager>().unwrap();

            asset_manager.update_asset_loaders().unwrap();
        }

        //
        // Notify egui of frame begin
        //
        #[cfg(feature = "egui")]
        {
            let egui_manager = self.resources.get::<WinitEguiManager>().unwrap();
            egui_manager.begin_frame(window)?;
        }

        {
            self.scene_manager
                .update_scene(&mut self.world, &mut self.resources);
        }

        //
        // Close egui input for this frame
        //
        #[cfg(feature = "egui")]
        {
            let egui_manager = self.resources.get::<WinitEguiManager>().unwrap();
            egui_manager.end_frame();
        }

        let t1 = Instant::now();
        log::trace!(
            "[main] Simulation took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        profiling::finish_frame!();
        Ok(ControlFlow::Poll)
    }

    pub fn init(args: &ApiaryArgs, window: &winit::window::Window) -> ApiaryResult<Self> {
        profiling::register_thread!("Main Thread");

        let api = unsafe { RafxApi::new(window, &Default::default())? };

        let mut resources = Resources::default();
        resources.insert(TimeState::new());

        let scene_manager = SceneManager::default();

        let asset_source = args.asset_source().unwrap();

        let physical_size = window.inner_size();
        rendering_init(
            &mut resources,
            asset_source,
            window,
            physical_size.width,
            physical_size.height,
        )?;

        let font = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf")
        };

        let world = World::default();

        {}
        Ok(ApiaryApp {
            api,
            resources,
            world,
            scene_manager,
        })
    }

    pub fn shutdown(&mut self) -> ApiaryResult<()> {
        self.api.destroy()?;
        Ok(())
    }
}

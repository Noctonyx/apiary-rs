use crate::error::ApiaryResult;
use crate::input;
use crate::input::InputResource;
use crate::rendering::{rendering_destroy, rendering_init};
use crate::scenes::{create_scene, SceneManager};
use crate::time::{PeriodicEvent, TimeState};
use legion::{Resources, World};
//use puffin_egui::puffin;
use rafx::api::{RafxApi, RafxExtents2D, RafxSwapchainHelper};
use rafx::assets::distill::loader::handle::Handle;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::framework::render_features::ExtractResources;
use rafx::renderer::daemon::AssetDaemonOpt;
use rafx::renderer::{AssetSource, Renderer, RendererConfigResource, ViewportsResource};
use rafx::visibility::VisibilityRegion;
use rafx_plugins::assets::font::FontAsset;
use rafx_plugins::features::egui::{EguiContextResource, WinitEguiManager};
use rafx_plugins::features::mesh_basic::MeshBasicRenderOptions;
use rafx_plugins::features::skybox::SkyboxResource;
use rafx_plugins::features::text::TextResource;
use rafx_plugins::features::tile_layer::TileLayerResource;
use rafx_plugins::pipelines::basic::{
    BasicPipelineRenderOptions, BasicPipelineTonemapDebugData, BasicPipelineTonemapperType,
};
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

    print_time_event: PeriodicEvent,
    font: Handle<FontAsset>,
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

#[derive(Clone)]
pub struct RenderOptions {
    pub enable_msaa: bool,
    pub enable_hdr: bool,
    pub enable_bloom: bool,
    pub enable_textures: bool,
    pub enable_lighting: bool,
    pub show_surfaces: bool,
    pub show_wireframes: bool,
    pub show_debug3d: bool,
    pub show_text: bool,
    pub show_skybox: bool,
    pub show_feature_toggles: bool,
    pub show_shadows: bool,
    pub blur_pass_count: usize,
    pub tonemapper_type: BasicPipelineTonemapperType,
    pub enable_visibility_update: bool,
}

impl RenderOptions {
    fn default_2d() -> Self {
        RenderOptions {
            enable_msaa: false,
            enable_hdr: false,
            enable_bloom: false,
            enable_textures: true,
            enable_lighting: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_shadows: true,
            show_feature_toggles: false,
            blur_pass_count: 0,
            tonemapper_type: BasicPipelineTonemapperType::None,
            enable_visibility_update: true,
        }
    }

    pub fn default_3d() -> Self {
        RenderOptions {
            enable_msaa: true,
            enable_hdr: false,
            enable_bloom: true,
            enable_textures: true,
            enable_lighting: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_shadows: true,
            show_feature_toggles: true,
            blur_pass_count: 5,
            tonemapper_type: BasicPipelineTonemapperType::AutoExposureOld,
            enable_visibility_update: true,
        }
    }
}

impl RenderOptions {
    #[cfg(feature = "egui")]
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.enable_msaa, "enable_msaa");
        ui.checkbox(&mut self.enable_hdr, "enable_hdr");

        if self.enable_hdr {
            ui.indent("HDR options", |ui| {
                let tonemapper_names: Vec<_> = (0..(BasicPipelineTonemapperType::MAX as i32))
                    .map(|t| BasicPipelineTonemapperType::from(t).display_name())
                    .collect();

                egui::ComboBox::from_label("tonemapper_type")
                    .selected_text(tonemapper_names[self.tonemapper_type as usize])
                    .show_ui(ui, |ui| {
                        for (i, name) in tonemapper_names.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.tonemapper_type,
                                BasicPipelineTonemapperType::from(i as i32),
                                name,
                            );
                        }
                    });

                ui.checkbox(&mut self.enable_bloom, "enable_bloom");
                if self.enable_bloom {
                    ui.indent("", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.blur_pass_count, 0..=10)
                                .clamp_to_range(true)
                                .text("blur_pass_count"),
                        );
                    });
                }
            });
        }

        if self.show_feature_toggles {
            ui.checkbox(&mut self.show_wireframes, "show_wireframes");
            ui.checkbox(&mut self.show_surfaces, "show_surfaces");

            if self.show_surfaces {
                ui.indent("", |ui| {
                    ui.checkbox(&mut self.enable_textures, "enable_textures");
                    ui.checkbox(&mut self.enable_lighting, "enable_lighting");

                    if self.enable_lighting {
                        ui.indent("", |ui| {
                            ui.checkbox(&mut self.show_shadows, "show_shadows");
                        });
                    }

                    ui.checkbox(&mut self.show_skybox, "show_skybox_feature");
                });
            }

            ui.checkbox(&mut self.show_debug3d, "show_debug3d_feature");
            ui.checkbox(&mut self.show_text, "show_text_feature");
        }

        ui.checkbox(
            &mut self.enable_visibility_update,
            "enable_visibility_update",
        );
    }
}

#[derive(Default)]
pub struct DebugUiState {
    show_render_options: bool,
    show_asset_list: bool,
    show_tonemap_debug: bool,

    #[cfg(feature = "profile-with-puffin")]
    show_profiler: bool,
}

impl ApiaryApp {
    //#[profiling::function]
    pub fn update(
        &mut self,
        window: &winit::window::Window,
    ) -> ApiaryResult<winit::event_loop::ControlFlow> {
        profiling::scope!("Main Loop");
        //profiling::
        //puffin::GlobalProfiler::lock().new_frame();

        let t0 = Instant::now();

        //
        // Update time
        //
        {
            self.resources.get_mut::<TimeState>().unwrap().update();
        }

        //
        // Print FPS
        //
        {
            let time_state = self.resources.get::<TimeState>().unwrap();
            if self.print_time_event.try_take_event(
                time_state.current_instant(),
                std::time::Duration::from_secs_f32(1.0),
            ) {
                log::info!("FPS: {}", time_state.updates_per_second());
                //renderer.dump_stats();
            }
        }

        {
            let mut viewports = self.resources.get_mut::<ViewportsResource>().unwrap();
            let physical_size = window.inner_size();
            viewports.main_window_size = RafxExtents2D {
                width: physical_size.width,
                height: physical_size.height,
            };
        }

        {
            if self.scene_manager.has_next_scene() {
                self.scene_manager
                    .try_cleanup_current_scene(&mut self.world, &self.resources);

                {
                    // NOTE(dvd): Legion leaks memory because the entity IDs aren't reset when the
                    // world is cleared and the entity location map will grow without bounds.
                    self.world = World::default();

                    // NOTE(dvd): The Renderer maintains some per-frame temporary data to avoid
                    // allocating each frame. We can clear this between scene transitions.
                    let mut renderer = self.resources.get_mut::<Renderer>().unwrap();
                    renderer.clear_temporary_work();
                }

                //self.scene_manager.go_next_scene();
                //.try_create_next_scene(&mut self.world, &self.resources);
            }
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
            let mut text_resource = self.resources.get_mut::<TextResource>().unwrap();

            text_resource.add_text(
                "Use Left/Right arrow keys to switch demos".to_string(),
                glam::Vec3::new(100.0, 400.0, 0.0),
                &self.font,
                20.0,
                glam::Vec4::new(1.0, 1.0, 1.0, 1.0),
            );
        }

        {
            self.scene_manager
                .update_scene(&mut self.world, &mut self.resources);
        }

        #[cfg(feature = "egui")]
        {
            let ctx = self
                .resources
                .get::<EguiContextResource>()
                .unwrap()
                .context();
            let time_state = self.resources.get::<TimeState>().unwrap();
            let mut debug_ui_state = self.resources.get_mut::<DebugUiState>().unwrap();
            let mut render_options = self.resources.get_mut::<RenderOptions>().unwrap();
            let tonemap_debug_data = self
                .resources
                .get::<BasicPipelineTonemapDebugData>()
                .unwrap();
            let asset_manager = self.resources.get::<AssetResource>().unwrap();

            egui::TopBottomPanel::top("top_panel").show(&ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "Windows", |ui| {
                        ui.checkbox(&mut debug_ui_state.show_render_options, "Render Options");

                        ui.checkbox(&mut debug_ui_state.show_asset_list, "Asset List");
                        ui.checkbox(&mut debug_ui_state.show_tonemap_debug, "Tonemap Debug");

                        #[cfg(feature = "profile-with-puffin")]
                        if ui
                            .checkbox(&mut debug_ui_state.show_profiler, "Profiler")
                            .changed()
                        {
                            log::info!(
                                "Setting puffin profiler enabled: {:?}",
                                debug_ui_state.show_profiler
                            );
                            profiling::puffin::set_scopes_on(debug_ui_state.show_profiler);
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.label(format!("Frame: {}", time_state.update_count()));
                        ui.separator();
                        ui.label(format!(
                            "FPS: {:.1}",
                            time_state.updates_per_second_smoothed()
                        ));
                    });
                })
            });

            if debug_ui_state.show_tonemap_debug {
                egui::Window::new("Tonemap Debug")
                    .open(&mut debug_ui_state.show_tonemap_debug)
                    .show(&ctx, |ui| {
                        let data = tonemap_debug_data.inner.lock().unwrap();

                        ui.add(egui::Label::new(format!(
                            "histogram_sample_count: {}",
                            data.histogram_sample_count
                        )));
                        ui.add(egui::Label::new(format!(
                            "histogram_max_value: {}",
                            data.histogram_max_value
                        )));

                        use egui::plot::{Line, Plot, VLine, Value, Values};
                        let line_values: Vec<_> = data
                            .histogram
                            .iter()
                            //.skip(1) // don't include index 0
                            .enumerate()
                            .map(|(i, value)| Value::new(i as f64, *value as f64))
                            .collect();
                        let line =
                            Line::new(Values::from_values_iter(line_values.into_iter())).fill(0.0);
                        let average_line = VLine::new(data.result_average_bin);
                        let low_line = VLine::new(data.result_low_bin);
                        let high_line = VLine::new(data.result_high_bin);
                        Some(
                            ui.add(
                                Plot::new("my_plot")
                                    .line(line)
                                    .vline(average_line)
                                    .vline(low_line)
                                    .vline(high_line)
                                    .include_y(0.0)
                                    .include_y(1.0)
                                    .show_axes([false, false]),
                            ),
                        )
                    });
            }

            tonemap_debug_data
                .inner
                .lock()
                .unwrap()
                .enable_debug_data_collection = debug_ui_state.show_tonemap_debug;

            if debug_ui_state.show_render_options {
                egui::Window::new("Render Options")
                    .open(&mut debug_ui_state.show_render_options)
                    .show(&ctx, |ui| {
                        render_options.ui(ui);
                    });
            }

            if debug_ui_state.show_asset_list {
                egui::Window::new("Asset List")
                    .open(&mut debug_ui_state.show_asset_list)
                    .show(&ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let loader = asset_manager.loader();
                            let mut asset_info = loader
                                .get_active_loads()
                                .into_iter()
                                .map(|item| loader.get_load_info(item))
                                .collect::<Vec<_>>();
                            asset_info.sort_by(|x, y| {
                                x.as_ref()
                                    .map(|x| &x.path)
                                    .cmp(&y.as_ref().map(|y| &y.path))
                            });
                            for info in asset_info {
                                if let Some(info) = info {
                                    let id = info.asset_id;
                                    ui.label(format!(
                                        "{}:{} .. {}",
                                        info.file_name.unwrap_or_else(|| "???".to_string()),
                                        info.asset_name.unwrap_or_else(|| format!("{}", id)),
                                        info.refs
                                    ));
                                } else {
                                    ui.label("NO INFO");
                                }
                            }
                        });
                    });
            }

            #[cfg(feature = "profile-with-puffin")]
            if debug_ui_state.show_profiler {
                profiling::scope!("puffin profiler");
                puffin_egui::profiler_window(&ctx);
            }

            let mut render_config_resource =
                self.resources.get_mut::<RendererConfigResource>().unwrap();
            render_config_resource
                .visibility_config
                .enable_visibility_update = render_options.enable_visibility_update;
        }

        {
            let render_options = self.resources.get::<RenderOptions>().unwrap();

            let mut bpro = self
                .resources
                .get_mut::<BasicPipelineRenderOptions>()
                .unwrap();
            bpro.enable_msaa = render_options.enable_msaa;
            bpro.enable_hdr = render_options.enable_hdr;
            bpro.enable_bloom = render_options.enable_bloom;
            bpro.enable_textures = render_options.enable_textures;
            bpro.show_surfaces = render_options.show_surfaces;
            bpro.show_wireframes = render_options.show_wireframes;
            bpro.show_debug3d = render_options.show_debug3d;
            bpro.show_text = render_options.show_text;
            bpro.show_skybox = render_options.show_skybox;
            bpro.show_feature_toggles = render_options.show_feature_toggles;
            bpro.blur_pass_count = render_options.blur_pass_count;
            bpro.tonemapper_type = render_options.tonemapper_type;
            bpro.enable_visibility_update = render_options.enable_visibility_update;

            let mut mesh_render_options =
                self.resources.get_mut::<MeshBasicRenderOptions>().unwrap();
            mesh_render_options.show_surfaces = render_options.show_surfaces;
            mesh_render_options.show_shadows = render_options.show_shadows;
            mesh_render_options.enable_lighting = render_options.enable_lighting;
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

        //
        // Redraw
        //
        {
            let dt = self
                .resources
                .get::<TimeState>()
                .unwrap()
                .previous_update_time();

            profiling::scope!("Start Next Frame Render");
            let renderer = self.resources.get::<Renderer>().unwrap();

            let mut extract_resources = ExtractResources::default();

            macro_rules! add_to_extract_resources {
                ($ty: ident) => {
                    #[allow(non_snake_case)]
                    let mut $ty = self.resources.get_mut::<$ty>().unwrap();
                    extract_resources.insert(&mut *$ty);
                };
                ($ty: path, $name: ident) => {
                    let mut $name = self.resources.get_mut::<$ty>().unwrap();
                    extract_resources.insert(&mut *$name);
                };
            }

            add_to_extract_resources!(VisibilityRegion);
            add_to_extract_resources!(RafxSwapchainHelper);
            add_to_extract_resources!(ViewportsResource);
            add_to_extract_resources!(AssetManager);
            add_to_extract_resources!(TimeState);
            add_to_extract_resources!(RenderOptions);
            add_to_extract_resources!(BasicPipelineRenderOptions);
            add_to_extract_resources!(BasicPipelineTonemapDebugData);
            add_to_extract_resources!(MeshBasicRenderOptions);
            add_to_extract_resources!(RendererConfigResource);
            add_to_extract_resources!(TileLayerResource);
            add_to_extract_resources!(SkyboxResource);
            add_to_extract_resources!(
                rafx_plugins::features::sprite::SpriteRenderObjectSet,
                sprite_render_object_set
            );
            add_to_extract_resources!(
                rafx_plugins::features::mesh_basic::MeshBasicRenderObjectSet,
                mesh_render_object_set
            );
            add_to_extract_resources!(
                rafx_plugins::features::tile_layer::TileLayerRenderObjectSet,
                tile_layer_render_object_set
            );
            add_to_extract_resources!(
                rafx_plugins::features::debug3d::Debug3DResource,
                debug_draw_3d_resource
            );
            add_to_extract_resources!(rafx_plugins::features::text::TextResource, text_resource);

            #[cfg(feature = "egui")]
            add_to_extract_resources!(
                rafx_plugins::features::egui::WinitEguiManager,
                winit_egui_manager
            );

            extract_resources.insert(&mut self.world);

            renderer
                .start_rendering_next_frame(&mut extract_resources, dt)
                .unwrap();
        }

        let t2 = rafx::base::Instant::now();
        log::trace!(
            "[main] start rendering took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        profiling::finish_frame!();
        Ok(ControlFlow::Poll)
    }

    pub fn init(args: &ApiaryArgs, window: &winit::window::Window) -> ApiaryResult<Self> {
        profiling::register_thread!("Main Thread");

        let api = unsafe { RafxApi::new(window, &Default::default())? };

        let mut resources = Resources::default();
        resources.insert(TimeState::new());
        resources.insert(InputResource::new());
        resources.insert(RenderOptions::default_2d());
        resources.insert(MeshBasicRenderOptions::default());
        resources.insert(BasicPipelineRenderOptions::default());
        resources.insert(BasicPipelineTonemapDebugData::default());
        resources.insert(DebugUiState::default());

        let mut scene_manager = SceneManager::default();

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

        let mut world = World::default();
        scene_manager.set_scene(create_scene(&mut world, &resources));

        let print_time_event = crate::time::PeriodicEvent::default();

        {}
        Ok(ApiaryApp {
            api,
            resources,
            world,
            scene_manager,
            print_time_event,
            font,
        })
    }

    pub fn shutdown(&mut self) -> ApiaryResult<()> {
        self.api.destroy()?;
        Ok(())
    }

    pub fn process_input(
        &mut self,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    ) -> bool {
        Self::do_process_input(
            &mut self.scene_manager,
            &mut self.world,
            &self.resources,
            event,
            window,
        )
    }

    fn do_process_input(
        scene_manager: &mut SceneManager,
        world: &mut World,
        resources: &Resources,
        event: &winit::event::Event<()>,
        _window: &winit::window::Window,
    ) -> bool {
        use winit::event::*;

        #[cfg(feature = "egui")]
        let egui_manager = resources
            .get::<rafx_plugins::features::egui::WinitEguiManager>()
            .unwrap();

        #[cfg(feature = "egui")]
        let ignore_event = {
            egui_manager.handle_event(event);
            egui_manager.ignore_event(event)
        };

        #[cfg(not(feature = "egui"))]
        let ignore_event = false;

        if !ignore_event {
            //log::trace!("{:?}", event);
            let mut was_handled = false;
            match event {
                //
                // Halt if the user requests to close the window
                //
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => return false,

                //
                // Close if the escape key is hit
                //
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(virtual_keycode),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    //log::trace!("Key Down {:?} {:?}", keycode, modifiers);
                    if *virtual_keycode == VirtualKeyCode::Escape {
                        return false;
                    }

                    if *virtual_keycode == VirtualKeyCode::M {
                        let metrics = resources.get::<AssetManager>().unwrap().metrics();
                        println!("{:#?}", metrics);
                        was_handled = true;
                    }
                }
                _ => {}
            }

            if !was_handled {
                scene_manager.process_input(world, resources, event);

                {
                    let mut input_resource = resources.get_mut::<InputResource>().unwrap();
                    input::handle_winit_event(event, &mut *input_resource);
                }
            }
        }

        true
    }
}

impl Drop for ApiaryApp {
    fn drop(&mut self) {
        rendering_destroy(&mut self.resources).unwrap()
    }
}

mod ui_scene;

use legion::{IntoQuery, Read, Resources, World};
use rafx_plugins::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use rafx_plugins::features::debug3d::Debug3DResource;
use ui_scene::UiScene;

pub struct SceneManager {
    current_scene: Option<Box<dyn Scene>>,
    next_scene: Option<Box<dyn Scene>>,
}

pub trait Scene {
    fn update(&mut self, world: &mut World, resources: &mut Resources);
    fn cleanup(&mut self, _world: &mut World, _resources: &Resources) {}
    fn process_input(
        &mut self,
        _world: &mut World,
        _resources: &Resources,
        _event: &winit::event::Event<()>,
    ) {
    }
}

impl SceneManager {
    pub fn set_scene(&mut self, scene: Box<dyn Scene>) {
        self.current_scene = Some(scene);
    }

    pub fn update_scene(&mut self, world: &mut World, resources: &mut Resources) {
        if self.current_scene.is_none() {
            return;
        }
        self.current_scene
            .as_mut()
            .unwrap()
            .update(world, resources);
    }

    pub fn process_input(
        &mut self,
        world: &mut World,
        resources: &Resources,
        event: &winit::event::Event<()>,
    ) {
        if let Some(current_scene) = &mut self.current_scene {
            current_scene.process_input(world, resources, event);
        }
    }

    pub fn has_next_scene(&self) -> bool {
        self.next_scene.is_some()
    }

    pub fn try_cleanup_current_scene(&mut self, world: &mut World, resources: &Resources) {
        if let Some(current_scene) = &mut self.current_scene {
            current_scene.cleanup(world, resources);
        }

        world.clear();
        self.current_scene = None;
    }
    /*
    pub fn go_next_scene(&mut self) {
        let mut n = self.next_scene.unwrap();
        self.current_scene = Some(n);
        self.next_scene = None;
    }
    */
}

impl Default for SceneManager {
    fn default() -> Self {
        SceneManager {
            current_scene: None,
            next_scene: None,
        }
    }
}

fn add_light_debug_draw(resources: &Resources, world: &World) {
    let mut debug_draw = resources.get_mut::<Debug3DResource>().unwrap();

    let mut query = <Read<DirectionalLightComponent>>::query();
    for light in query.iter(world) {
        let light_from = light.direction * -10.0;
        let light_to = glam::Vec3::ZERO;

        debug_draw.add_line(light_from, light_to, light.color);
    }

    let mut query = <(Read<TransformComponent>, Read<PointLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        debug_draw.add_sphere(transform.translation, 0.1, light.color, 12);
        debug_draw.add_sphere(transform.translation, light.range, light.color, 12);
    }

    let mut query = <(Read<TransformComponent>, Read<SpotLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        let light_from = transform.translation;
        let light_to = transform.translation + light.direction;
        let light_direction = (light_to - light_from).normalize();

        debug_draw.add_cone(
            light_from,
            light_from + (light.range * light_direction),
            light.range * light.spotlight_half_angle.tan(),
            light.color,
            10,
        );
    }
}

fn add_directional_light(
    _resources: &Resources,
    world: &mut World,
    light_component: DirectionalLightComponent,
) {
    world.extend(vec![(light_component,)]);
}

fn add_spot_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: SpotLightComponent,
) {
    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    world.extend(vec![(position_component, light_component)]);
}

fn add_point_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: PointLightComponent,
) {
    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    world.extend(vec![(position_component, light_component)]);
}

pub fn create_scene(world: &mut World, resources: &Resources) -> Box<dyn Scene> {
    Box::new(UiScene::new(world, resources))
}

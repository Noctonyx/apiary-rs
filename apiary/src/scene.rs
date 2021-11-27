use legion::{Resources, World};

pub struct SceneManager {
    current_scene: Option<Box<dyn Scene>>,
}

trait Scene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    );
}

impl SceneManager {
    pub fn update_scene(&mut self,
                        world: &mut World,
                        resources: &mut Resources) {
        self.current_scene
            .as_mut()
            .unwrap()
            .update(world, resources);
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        SceneManager {
            current_scene: None,
        }
    }
}
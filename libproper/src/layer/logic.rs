use std::sync::{atomic::Ordering, Arc, Mutex};

use nalgebra::{Point3, Vector3};
use vulkano::sync::GpuFuture;
use winit::event_loop::{ControlFlow, EventLoopProxy};

use crate::{
    error::Error,
    event::{Event, GameEvent},
    render::frame::Frame,
    resource::{
        material::{MaterialInstanceCreateInfo, MaterialRegistry},
        model::ModelRegistry,
        texture::TextureRegistry,
    },
    world::{entity::Entity, scene::Scene},
};

use super::{input::InputState, Layer};

pub struct LogicLayer {
    #[allow(dead_code)]
    event_proxy: EventLoopProxy<GameEvent>,
    scene: Arc<Mutex<Scene>>,
    material_registry: Arc<Mutex<MaterialRegistry>>,
    model_registry: Arc<Mutex<ModelRegistry>>,
    texture_registry: Arc<Mutex<TextureRegistry>>,
    input_state: Arc<InputState>,
}

impl LogicLayer {
    pub fn new(
        event_proxy: EventLoopProxy<GameEvent>,
        scene: Arc<Mutex<Scene>>,
        material_registry: Arc<Mutex<MaterialRegistry>>,
        model_registry: Arc<Mutex<ModelRegistry>>,
        texture_registry: Arc<Mutex<TextureRegistry>>,
        input_state: Arc<InputState>,
    ) -> Self {
        Self {
            event_proxy,
            scene,
            material_registry,
            model_registry,
            texture_registry,
            input_state,
        }
    }

    pub fn test_event(&self) -> Result<(), Error> {
        let mut materials = self.material_registry.lock().unwrap();
        let mut models = self.model_registry.lock().unwrap();
        let mut textures = self.texture_registry.lock().unwrap();
        let mut scene = self.scene.lock().unwrap();

        let position = random_point() * 4.0;
        let model_type = rand::random();
        let texture_type = rand::random();

        let material = materials.get_or_load("simple").unwrap();
        let texture = if texture_type {
            textures.get_or_load("texture0")?
        } else {
            textures.get_or_load("texture1")?
        };
        let material_create_info = MaterialInstanceCreateInfo::default()
            .with_color("diffuse_color", [0.0, 1.0, 0.0, 1.0])
            .with_texture("diffuse_map", texture);
        let mesh = if model_type {
            models.create_mesh_object("torus", material, material_create_info)?
        } else {
            models.create_mesh_object("monkey", material, material_create_info)?
        };

        let entity = Entity::new_with_mesh(position, mesh)?;

        scene.add(entity);

        Ok(())
    }
}

impl Layer for LogicLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        _frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error> {
        Ok(in_future)
    }

    fn on_tick(&mut self, delta: f64) -> Result<(), Error> {
        let want_forward = i32::from(self.input_state.forward.load(Ordering::Acquire))
            - i32::from(self.input_state.back.load(Ordering::Acquire));
        let want_side = i32::from(self.input_state.right.load(Ordering::Acquire))
            - i32::from(self.input_state.left.load(Ordering::Acquire));
        let want_vertical = i32::from(self.input_state.up.load(Ordering::Acquire))
            - i32::from(self.input_state.down.load(Ordering::Acquire));

        if want_forward != 0 || want_side != 0 || want_vertical != 0 {
            let mut scene = self.scene.lock().unwrap();
            let real_forward = scene.camera.forward();
            let real_sideward = scene.camera.sideward();
            let forward = Vector3::new(real_forward.x, 0.0, real_forward.z) * (want_forward as f32);
            let sideward = Vector3::new(real_sideward.x, 0.0, real_sideward.z) * (want_side as f32);
            let vertical = Vector3::new(0.0, want_vertical as f32, 0.0);
            let delta = (forward + sideward + vertical).normalize() * (delta as f32) * 2.0;

            scene.camera.translate(delta);
        }

        Ok(())
    }

    fn on_event(&mut self, event: &Event, _flow: &mut ControlFlow) -> Result<bool, Error> {
        if let Event::MouseMotion(delta) = event {
            let mut scene = self.scene.lock().unwrap();
            scene
                .camera
                .rotate_angles(-delta.1 as f32 * 0.02, delta.0 as f32 * 0.02);
            return Ok(true);
        }
        if let Event::GameEvent(GameEvent::TestEvent) = event {
            self.test_event()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn random_point() -> Point3<f32> {
    let x = (rand::random::<f32>() - 0.5) * 2.0;
    let y = (rand::random::<f32>() - 0.5) * 2.0;
    let z = (rand::random::<f32>() - 0.5) * 2.0;
    Point3::new(x, y, z)
}

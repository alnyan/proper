use std::sync::{Arc, Mutex};

use nalgebra::Point3;
use vulkano::sync::GpuFuture;
use winit::event_loop::ControlFlow;

use crate::{
    error::Error,
    event::{Event, GameEvent},
    render::frame::Frame,
    resource::{
        material::{MaterialInstanceCreateInfo, MaterialRegistry},
        model::ModelRegistry, texture::TextureRegistry,
    },
    world::{entity::Entity, scene::Scene},
};

use super::Layer;

pub struct LogicLayer {
    scene: Arc<Mutex<Scene>>,
    material_registry: Arc<Mutex<MaterialRegistry>>,
    model_registry: Arc<Mutex<ModelRegistry>>,
    texture_registry: Arc<Mutex<TextureRegistry>>
}

impl LogicLayer {
    pub fn new(
        scene: Arc<Mutex<Scene>>,
        material_registry: Arc<Mutex<MaterialRegistry>>,
        model_registry: Arc<Mutex<ModelRegistry>>,
        texture_registry: Arc<Mutex<TextureRegistry>>
    ) -> Self {
        Self {
            scene,
            material_registry,
            model_registry,
            texture_registry
        }
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

    fn on_event(&mut self, event: &Event, _flow: &mut ControlFlow) -> Result<bool, Error> {
        if let Event::GameEvent(GameEvent::TestEvent) = event {
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

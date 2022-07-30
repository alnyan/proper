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
        model::Model,
    },
    world::{
        entity::{Entity, MeshParameters},
        scene::{MeshObject, Scene},
    },
};

use super::Layer;

pub struct LogicLayer {
    scene: Arc<Mutex<Scene>>,
    material_registry: Arc<Mutex<MaterialRegistry>>,
}

impl LogicLayer {
    pub fn new(scene: Arc<Mutex<Scene>>, material_registry: Arc<Mutex<MaterialRegistry>>) -> Self {
        Self {
            scene,
            material_registry,
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
            let mut scene = self.scene.lock().unwrap();

            let mat_id = materials.get_or_load("simple").unwrap();
            let model = Model::host("res/models/torus.obj", mat_id);
            let entity = Entity::new_dynamic(
                Point3::new(0.0, 0.0, 0.0),
                MeshParameters {
                    model,
                    material_create_info: MaterialInstanceCreateInfo::default()
                        .with_color("diffuse_color", [0.0, 1.0, 0.0, 1.0]),
                },
            );

            scene.add(entity);

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

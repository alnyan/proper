use std::sync::Arc;

use bytemuck::Zeroable;
use nalgebra::Matrix4;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Queue,
};

use crate::{render::shader, resource::model::Model};

use super::entity::Entity;

pub struct Scene {
    pub entities: Vec<Entity>,
}

pub struct MeshObject {
    model: Arc<Model>,
    model_buffer: Arc<CpuAccessibleBuffer<shader::scene_vs::ty::Model_Data>>,
}

impl Scene {
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.iter_mut()
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self { entities: vec![] }
    }
}

impl MeshObject {
    pub fn new(gfx_queue: Arc<Queue>, model: Arc<Model>) -> Self {
        let model_buffer = CpuAccessibleBuffer::from_data(
            gfx_queue.device().clone(),
            BufferUsage::uniform_buffer(),
            false,
            Zeroable::zeroed(),
        )
        .unwrap();
        Self {
            model,
            model_buffer,
        }
    }

    pub const fn model(&self) -> &Arc<Model> {
        &self.model
    }

    pub const fn model_buffer(
        &self,
    ) -> &Arc<CpuAccessibleBuffer<shader::scene_vs::ty::Model_Data>> {
        &self.model_buffer
    }

    pub fn update_transform(&mut self, transform: &Matrix4<f32>) {
        let mut lock = self.model_buffer.write().unwrap();
        lock.transform = transform.as_ref().clone();
    }
}

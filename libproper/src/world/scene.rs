use std::sync::Arc;

use bytemuck::Zeroable;
use nalgebra::Matrix4;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Queue,
    sync::GpuFuture,
};

use crate::{
    error::Error,
    render::shader,
    resource::{
        material::{
            MaterialInstance, MaterialInstanceCreateInfo, MaterialRegistry, MaterialTemplateId,
        },
        model::Model,
    },
};

use super::entity::Entity;

#[derive(Default)]
pub struct Scene {
    // Renderable entities, sorted by material template
    data: Vec<MaterialEntityGroup>,
}

pub struct MaterialEntityGroup {
    material_template_id: MaterialTemplateId,
    entities: Vec<Entity>,
}

pub struct MeshObject {
    model: Arc<Model>,
    model_buffer: Arc<CpuAccessibleBuffer<shader::simple_vs::ty::Model_Data>>,
    material_instance: MaterialInstance,
}

impl Scene {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &MaterialEntityGroup> {
        self.data.iter()
    }
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut MaterialEntityGroup> {
        self.data.iter_mut()
    }

    pub fn add(&mut self, entity: Entity) {
        let material_template_id = entity.mesh().unwrap().model_material_template_id();

        if let Some(group) = self
            .data
            .iter_mut()
            .find(|p| p.material_template_id == material_template_id)
        {
            group.entities.push(entity);
        } else {
            self.data.push(MaterialEntityGroup {
                material_template_id,
                entities: vec![entity],
            });
        }
    }
}

impl MaterialEntityGroup {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    #[inline]
    pub const fn material_template_id(&self) -> MaterialTemplateId {
        self.material_template_id
    }
}

impl MeshObject {
    pub fn new(
        gfx_queue: Arc<Queue>,
        model: Arc<Model>,
        material_registry: &MaterialRegistry,
        material_instance_create_info: MaterialInstanceCreateInfo,
    ) -> Result<Self, Error> {
        let model_buffer = CpuAccessibleBuffer::from_data(
            gfx_queue.device().clone(),
            BufferUsage::uniform_buffer(),
            false,
            Zeroable::zeroed(),
        )?;

        let material_template = material_registry.get(model.material_template_id());
        let (material_instance, init) =
            material_template.create_instance(gfx_queue, material_instance_create_info)?;

        init.then_signal_fence_and_flush()?.wait(None).unwrap();

        Ok(Self {
            model,
            model_buffer,
            material_instance,
        })
    }

    #[inline]
    pub const fn model(&self) -> &Arc<Model> {
        &self.model
    }

    #[inline]
    pub const fn model_buffer(
        &self,
    ) -> &Arc<CpuAccessibleBuffer<shader::simple_vs::ty::Model_Data>> {
        &self.model_buffer
    }

    #[inline]
    pub fn model_material_template_id(&self) -> MaterialTemplateId {
        self.model.material_template_id()
    }

    pub const fn material_instance(&self) -> &MaterialInstance {
        &self.material_instance
    }

    pub fn update_transform(&mut self, transform: &Matrix4<f32>) -> Result<(), Error> {
        let mut lock = self.model_buffer.write()?;
        lock.transform = *transform.as_ref();
        Ok(())
    }
}

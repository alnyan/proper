use std::sync::{Arc, atomic::Ordering};

use bytemuck::Zeroable;
use nalgebra::Matrix4;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    pipeline::Pipeline,
    sync::GpuFuture,
};

use crate::{
    error::Error,
    render::shader,
    resource::{
        material::{MaterialInstance, MaterialInstanceCreateInfo, MaterialTemplate},
        model::Model,
    },
};

use super::{entity::Entity, camera::Camera};

#[derive(Default)]
pub struct Scene {
    // Renderable entities, sorted by material template
    pub camera: Camera,
    pub data: Vec<MaterialEntityGroup>,
    pub loading_list: Vec<Entity>,
}

pub struct MaterialEntityGroup {
    pub material_template: Arc<dyn MaterialTemplate>,
    pub entities: Vec<Entity>,
}

pub struct MeshObject {
    model: Arc<Model>,
    model_buffer: Arc<CpuAccessibleBuffer<shader::simple_vs::ty::Model_Data>>,
    model_set: Arc<PersistentDescriptorSet>,
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
        let material_template = entity.mesh().model().material_template();
        let id = material_template.id().load(Ordering::Acquire);

        if let Some(group) = self
            .data
            .iter_mut()
            .find(|p| id == p.material_template.id().load(Ordering::Acquire))
        {
            group.entities.push(entity);
        } else {
            self.data.push(MaterialEntityGroup {
                material_template: material_template.clone(),
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
}

impl MeshObject {
    pub fn new(
        gfx_queue: Arc<Queue>,
        model: Arc<Model>,
        material_template: Arc<dyn MaterialTemplate>,
        material_instance_create_info: MaterialInstanceCreateInfo,
    ) -> Result<Self, Error> {
        let model_buffer = CpuAccessibleBuffer::from_data(
            gfx_queue.device().clone(),
            BufferUsage::uniform_buffer(),
            false,
            Zeroable::zeroed(),
        )?;

        let pipeline_lock = material_template.pipeline().read().unwrap();
        let model_layout = pipeline_lock.layout().set_layouts().get(2).unwrap();
        let (material_instance, init) =
            material_template.create_instance(gfx_queue, material_instance_create_info)?;

        init.then_signal_fence_and_flush()?.wait(None).unwrap();

        let model_set = PersistentDescriptorSet::new(
            model_layout.clone(),
            vec![WriteDescriptorSet::buffer(0, model_buffer.clone())],
        )?;

        Ok(Self {
            model,
            model_buffer,
            model_set,
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
    pub const fn model_set(&self) -> &Arc<PersistentDescriptorSet> {
        &self.model_set
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

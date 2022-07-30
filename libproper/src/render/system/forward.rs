use rayon::prelude::*;
use std::{sync::{Arc, Mutex}, ops::{DerefMut, Deref}};

use vulkano::{
    buffer::TypedBufferAccess,
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo,
        CommandBufferInheritanceRenderPassInfo, CommandBufferInheritanceRenderPassType,
        CommandBufferUsage, PrimaryAutoCommandBuffer, SecondaryAutoCommandBuffer,
    },
    descriptor_set::PersistentDescriptorSet,
    device::Queue,
    pipeline::{graphics::viewport::Viewport, Pipeline, PipelineBindPoint, PipelineLayout},
    render_pass::{RenderPass, Subpass},
};

use crate::{
    error::Error,
    resource::material::{MaterialRegistry, MaterialTemplate, SimpleMaterial},
    world::{entity::Entity, scene::Scene},
};

pub struct ForwardSystem {
    gfx_queue: Arc<Queue>,
    common_pipeline_layout: Arc<PipelineLayout>,
    subpass: Subpass,
    material_registry: Arc<Mutex<MaterialRegistry>>,
}

impl ForwardSystem {
    pub fn new(
        gfx_queue: Arc<Queue>,
        viewport: &Viewport,
        subpass: Subpass,
        material_registry: Arc<Mutex<MaterialRegistry>>,
        common_pipeline_layout: Arc<PipelineLayout>,
    ) -> Result<Self, Error> {
        Ok(Self {
            gfx_queue,
            common_pipeline_layout,
            material_registry,
            subpass,
        })
    }

    pub const fn material_registry(&self) -> &Arc<Mutex<MaterialRegistry>> {
        &self.material_registry
    }

    fn record_command_buffer_part(
        &self,
        material_template: &dyn MaterialTemplate,
        scene_set: &Arc<PersistentDescriptorSet>,
        entities: &[Entity],
    ) -> SecondaryAutoCommandBuffer {
        let pipeline = material_template.pipeline();

        let mut secondary_builder = AutoCommandBufferBuilder::secondary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(CommandBufferInheritanceRenderPassType::BeginRenderPass(
                    CommandBufferInheritanceRenderPassInfo {
                        subpass: self.subpass.clone(),
                        framebuffer: None,
                    },
                )),
                ..Default::default()
            },
        )
        .unwrap();

        secondary_builder
            .bind_pipeline_graphics(pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.common_pipeline_layout.clone(),
                0,
                scene_set.clone(),
            );

        for object in entities {
            if let Some(mesh) = object.mesh() {
                let model = mesh.model();
                let model_data = model.data().unwrap();

                mesh.material_instance()
                    .bind_data(&mut secondary_builder, pipeline);

                secondary_builder
                    .bind_vertex_buffers(0, model_data.clone())
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        2,
                        mesh.model_set().clone(),
                    )
                    .draw(model_data.len().try_into().unwrap(), 1, 0, 0)
                    .unwrap();
            }
        }

        secondary_builder.build().unwrap()
    }

    fn record_secondary_buffers<T: Deref<Target = Scene>>(
        &self,
        scene_set: &Arc<PersistentDescriptorSet>,
        scene: T,
    ) -> Vec<SecondaryAutoCommandBuffer> {
        let mut cbs = vec![];

        let materials = self.material_registry.lock().unwrap();

        for group in scene.data.iter() {
            let num_objects = group.entities.len();
            let material_template = materials.get(group.material_template_id());
            if num_objects > 12 {
                let chunks = group.entities.chunks(num_objects / 12);

                let data: Vec<SecondaryAutoCommandBuffer> = chunks
                    .par_bridge()
                    .map(|chunk| self.record_command_buffer_part(material_template, scene_set, chunk))
                    .collect();

                cbs.extend(data);
            } else {
                cbs.push(self.record_command_buffer_part(material_template, scene_set, &group.entities));
            }
        }

        cbs
    }

    pub fn do_frame<T: Deref<Target = Scene>>(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        scene_set: &Arc<PersistentDescriptorSet>,
        scene: T,
    ) -> Result<(), Error> {
        let cbs = self.record_secondary_buffers(scene_set, scene);

        builder.execute_commands_from_vec(cbs).unwrap();

        Ok(())
    }

    pub fn swapchain_invalidated(&mut self, viewport: &Viewport) -> Result<(), Error> {
        self.material_registry.lock().unwrap().recreate_pipelines(
            viewport,
        )?;

        Ok(())
    }
}

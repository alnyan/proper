use rayon::prelude::*;
use std::sync::{Arc, Mutex};

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
    render_pass: Arc<RenderPass>,
    material_registry: Arc<Mutex<MaterialRegistry>>,
}

impl ForwardSystem {
    pub fn new(
        gfx_queue: Arc<Queue>,
        viewport: &Viewport,
        render_pass: Arc<RenderPass>,
        common_pipeline_layout: Arc<PipelineLayout>,
    ) -> Result<Self, Error> {
        let material_registry = Arc::new(Mutex::new(MaterialRegistry::default()));

        material_registry.lock().unwrap().add(
            "simple",
            Box::new(SimpleMaterial::new(&gfx_queue, &render_pass, viewport)?),
        );

        Ok(Self {
            gfx_queue,
            common_pipeline_layout,
            material_registry,
            render_pass,
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
                        subpass: Subpass::from(self.render_pass.clone(), 0).unwrap(),
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

    fn record_secondary_buffers(
        &self,
        scene_set: &Arc<PersistentDescriptorSet>,
        scene: &Scene,
    ) -> Vec<SecondaryAutoCommandBuffer> {
        let mut cbs = vec![];

        let materials = self.material_registry.lock().unwrap();

        for group in scene.data.iter() {
            let num_objects = group.entities.len();
            let material_template = materials.get(group.material_template_id());
            let chunks = group.entities.chunks(num_objects / 12);

            let data: Vec<SecondaryAutoCommandBuffer> = chunks
                .par_bridge()
                .map(|chunk| self.record_command_buffer_part(material_template, scene_set, chunk))
                .collect();

            cbs.extend(data);
        }

        cbs
    }

    pub fn do_frame(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        scene_set: &Arc<PersistentDescriptorSet>,
        scene: &Scene,
    ) -> Result<(), Error> {
        let cbs = self.record_secondary_buffers(scene_set, scene);

        builder.execute_commands_from_vec(cbs).unwrap();

        Ok(())
    }

    pub fn swapchain_invalidated(&mut self, viewport: &Viewport) -> Result<(), Error> {
        self.material_registry.lock().unwrap().recreate_pipelines(
            &self.gfx_queue,
            &self.render_pass,
            viewport,
        )?;

        Ok(())
    }
}

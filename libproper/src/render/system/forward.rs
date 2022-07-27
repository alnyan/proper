use rayon::prelude::*;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
    thread
};

use vulkano::{
    buffer::{BufferAccess, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo,
        CommandBufferInheritanceRenderPassInfo, CommandBufferInheritanceRenderPassType,
        CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
        SecondaryAutoCommandBuffer, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::{ClearValue, Format},
    image::{view::ImageView, AttachmentImage, ImageViewAbstract, SwapchainImage},
    pipeline::{graphics::viewport::Viewport, Pipeline, PipelineBindPoint, PipelineLayout},
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, query::QueryPipelineStatisticFlags,
};
use winit::window::Window;

use crate::{
    error::Error,
    render::frame::Frame,
    resource::material::{MaterialRegistry, SimpleMaterial, MaterialTemplate},
    world::{scene::Scene, entity::Entity},
};

type FramebufferCreateOutput = (Vec<Arc<Framebuffer>>, Arc<ImageView<AttachmentImage>>);

pub struct ForwardSystem {
    gfx_queue: Arc<Queue>,
    common_pipeline_layout: Arc<PipelineLayout>,
    framebuffers: Vec<Arc<Framebuffer>>,
    depth_view: Arc<ImageView<AttachmentImage>>,
    render_pass: Arc<RenderPass>,
    material_registry: Arc<Mutex<MaterialRegistry>>,
}

impl ForwardSystem {
    pub fn new(
        gfx_queue: Arc<Queue>,
        output_format: Format,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
        viewport: Viewport,
        common_pipeline_layout: Arc<PipelineLayout>,
    ) -> Result<Self, Error> {
        let render_pass = vulkano::single_pass_renderpass!(
            gfx_queue.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: output_format,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )?;

        let material_registry = Arc::new(Mutex::new(MaterialRegistry::default()));

        material_registry.lock().unwrap().add(
            "simple",
            Box::new(SimpleMaterial::new(&gfx_queue, &render_pass, &viewport)?),
        );

        let (framebuffers, depth_view) =
            Self::create_framebuffers(gfx_queue.device().clone(), &render_pass, swapchain_images)?;

        Ok(Self {
            gfx_queue,
            common_pipeline_layout,
            depth_view,
            framebuffers,
            material_registry,
            render_pass,
        })
    }

    pub const fn material_registry(&self) -> &Arc<Mutex<MaterialRegistry>> {
        &self.material_registry
    }

    fn record_command_buffer_part(&self, material_template: &dyn MaterialTemplate, scene_set: &Arc<PersistentDescriptorSet>, entities: &[Entity]) -> SecondaryAutoCommandBuffer {
        let t0 = Instant::now();
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

        secondary_builder.bind_pipeline_graphics(pipeline.clone()).bind_descriptor_sets(
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
                    .draw(model_data.len().try_into().unwrap(), 1, 0, 0).unwrap();
            }
        }

        let res = secondary_builder.build().unwrap();
        let t1 = Instant::now();
        //log::info!("Subbuffer record {:?} (n = {}): {:?}", thread::current().id(), entities.len(), t1 - t0);

        res
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

            let data: Vec<SecondaryAutoCommandBuffer> = chunks.par_bridge().map(|chunk| {
                self.record_command_buffer_part(material_template, scene_set, chunk)
            }).collect();

            cbs.extend(data);
        }

        cbs
    }

    pub fn do_frame(
        &mut self,
        scene_buffer: Arc<dyn BufferAccess>,
        frame: &Frame,
        scene: &Scene,
    ) -> Result<PrimaryAutoCommandBuffer, Error> {
        let scene_layout = self.common_pipeline_layout.set_layouts().get(0).unwrap();

        let scene_set = PersistentDescriptorSet::new(
            scene_layout.clone(),
            vec![WriteDescriptorSet::buffer(0, scene_buffer)],
        )?;

        let cbs = self.record_secondary_buffers(&scene_set, scene);

        let mut builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let framebuffer = self.framebuffers[frame.image_index].clone();
        let mut render_pass_begin_info = RenderPassBeginInfo::framebuffer(framebuffer);

        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])));
        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Depth(1.0)));

        builder
            .begin_render_pass(
                render_pass_begin_info,
                SubpassContents::SecondaryCommandBuffers,
            )?
            .execute_commands_from_vec(cbs)
            .unwrap()
            .end_render_pass()?;

        builder.build().map_err(Error::from)
    }

    pub fn swapchain_invalidated(
        &mut self,
        viewport: &Viewport,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) -> Result<(), Error> {
        (self.framebuffers, self.depth_view) = Self::create_framebuffers(
            self.gfx_queue.device().clone(),
            &self.render_pass,
            swapchain_images,
        )?;

        self.material_registry.lock().unwrap().recreate_pipelines(
            &self.gfx_queue,
            &self.render_pass,
            viewport,
        )?;

        Ok(())
    }

    fn create_framebuffers(
        device: Arc<Device>,
        render_pass: &Arc<RenderPass>,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) -> Result<FramebufferCreateOutput, Error> {
        let depth_view = ImageView::new_default(AttachmentImage::transient(
            device,
            swapchain_images[0].dimensions().width_height(),
            Format::D16_UNORM,
        )?)?;

        Ok((
            swapchain_images
                .into_iter()
                .map(|image| {
                    Framebuffer::new(
                        render_pass.clone(),
                        FramebufferCreateInfo {
                            attachments: vec![image.clone(), depth_view.clone()],
                            ..Default::default()
                        },
                    )
                })
                .collect::<Result<_, _>>()
                .map_err(Error::from)?,
            depth_view,
        ))
    }
}

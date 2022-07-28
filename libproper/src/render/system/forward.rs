use nalgebra::{Point3, Vector3};
use rayon::prelude::*;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use vulkano::{
    buffer::{BufferAccess, BufferUsage, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo,
        CommandBufferInheritanceRenderPassInfo, CommandBufferInheritanceRenderPassType,
        CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
        SecondaryAutoCommandBuffer, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::{ClearValue, Format},
    image::{view::ImageView, AttachmentImage, ImageViewAbstract, SampleCount, SwapchainImage},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    sync::GpuFuture,
};
use winit::window::Window;

use crate::{
    error::Error,
    render::{frame::Frame, shader, Vertex},
    resource::material::{MaterialRegistry, MaterialTemplate, SimpleMaterial},
    world::{entity::Entity, scene::Scene},
};

pub struct ForwardSystem {
    gfx_queue: Arc<Queue>,
    common_pipeline_layout: Arc<PipelineLayout>,
    render_pass: Arc<RenderPass>,
    material_registry: Arc<Mutex<MaterialRegistry>>,
    // TODO split off render subpasses into different structs
    screen_vertex_buffer: Arc<ImmutableBuffer<[Vertex]>>,
    screen_vs: Arc<ShaderModule>,
    screen_fs: Arc<ShaderModule>,
    screen_pipeline: Arc<GraphicsPipeline>,
}

impl ForwardSystem {
    pub fn new(
        gfx_queue: Arc<Queue>,
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
        common_pipeline_layout: Arc<PipelineLayout>,
    ) -> Result<Self, Error> {
        let material_registry = Arc::new(Mutex::new(MaterialRegistry::default()));

        material_registry.lock().unwrap().add(
            "simple",
            Box::new(SimpleMaterial::new(&gfx_queue, &render_pass, &viewport)?),
        );

        let (screen_vertex_buffer, init) = ImmutableBuffer::from_iter(
            vec![
                Vertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                    v_normal: Vector3::new(0.0, 0.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, -1.0, 0.0),
                    v_normal: Vector3::new(0.0, 0.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, 1.0, 0.0),
                    v_normal: Vector3::new(0.0, 0.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, 1.0, 0.0),
                    v_normal: Vector3::new(0.0, 0.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(-1.0, 1.0, 0.0),
                    v_normal: Vector3::new(0.0, 0.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                    v_normal: Vector3::new(0.0, 0.0, 0.0),
                },
            ],
            BufferUsage::vertex_buffer(),
            gfx_queue.clone(),
        )?;

        init.then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        let screen_vs = shader::screen_vs::load(gfx_queue.device().clone()).unwrap();
        let screen_fs = shader::screen_fs::load(gfx_queue.device().clone()).unwrap();

        let screen_pipeline = Self::create_screen_pipeline(
            gfx_queue.device().clone(),
            viewport.clone(),
            render_pass.clone(),
            screen_vs.clone(),
            screen_fs.clone(),
        );

        Ok(Self {
            gfx_queue,
            screen_vertex_buffer,
            common_pipeline_layout,
            material_registry,
            render_pass,
            screen_vs,
            screen_fs,
            screen_pipeline,
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

            let data: Vec<SecondaryAutoCommandBuffer> = chunks
                .par_bridge()
                .map(|chunk| self.record_command_buffer_part(material_template, scene_set, chunk))
                .collect();

            cbs.extend(data);
        }

        cbs
    }

    pub fn do_frame(
        &mut self,
        scene_buffer: Arc<dyn BufferAccess>,
        color_buffer: Arc<ImageView<AttachmentImage>>,
        framebuffer: Arc<Framebuffer>,
        frame: &Frame,
        scene: &Scene,
    ) -> Result<PrimaryAutoCommandBuffer, Error> {
        let scene_layout = self.common_pipeline_layout.set_layouts().get(0).unwrap();

        let scene_set = PersistentDescriptorSet::new(
            scene_layout.clone(),
            vec![WriteDescriptorSet::buffer(0, scene_buffer)],
        )?;

        let screen_layout = self.screen_pipeline.layout().set_layouts().get(0).unwrap();

        let screen_set = PersistentDescriptorSet::new(screen_layout.clone(), vec![
            WriteDescriptorSet::image_view(0, color_buffer)
        ])?;

        let t0 = Instant::now();
        let cbs = self.record_secondary_buffers(&scene_set, scene);

        let mut builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let mut render_pass_begin_info = RenderPassBeginInfo::framebuffer(framebuffer);

        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])));
        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Depth(1.0)));
        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])));

        builder
            .begin_render_pass(
                render_pass_begin_info,
                SubpassContents::SecondaryCommandBuffers,
            )?
            .execute_commands_from_vec(cbs)
            .unwrap()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.screen_pipeline.clone())
            .bind_vertex_buffers(0, self.screen_vertex_buffer.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.screen_pipeline.layout().clone(),
                0,
                screen_set,
            )
            .draw(6, 1, 0, 0)
            .unwrap()
            .end_render_pass()?;

        let res = builder.build().map_err(Error::from);

        let t1 = Instant::now();
        dbg!(t1 - t0);

        res
    }

    pub fn swapchain_invalidated(
        &mut self,
        viewport: &Viewport,

        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) -> Result<(), Error> {
        self.material_registry.lock().unwrap().recreate_pipelines(
            &self.gfx_queue,
            &self.render_pass,
            viewport,
        )?;
        self.screen_pipeline = Self::create_screen_pipeline(
            self.gfx_queue.device().clone(),
            viewport.clone(),
            self.render_pass.clone(),
            self.screen_vs.clone(),
            self.screen_fs.clone(),
        );

        Ok(())
    }

    fn create_screen_pipeline(
        device: Arc<Device>,
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
        screen_vs: Arc<ShaderModule>,
        screen_fs: Arc<ShaderModule>,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .input_assembly_state(InputAssemblyState::new())
            .render_pass(Subpass::from(render_pass, 1).unwrap())
            .vertex_shader(screen_vs.entry_point("main").unwrap(), ())
            .fragment_shader(screen_fs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .build(device)
            .unwrap()
    }
}

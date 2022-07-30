use std::sync::Arc;

use nalgebra::Point3;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    image::{view::ImageView, AttachmentImage},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{RenderPass, Subpass},
    shader::ShaderModule,
    sync::GpuFuture,
};

use crate::{
    error::Error,
    render::{shader, SimpleVertex},
};

pub struct ScreenSystem {
    gfx_queue: Arc<Queue>,
    subpass: Subpass,

    vertex_buffer: Arc<ImmutableBuffer<[SimpleVertex]>>,
    screen_set: Arc<PersistentDescriptorSet>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    pipeline: Arc<GraphicsPipeline>,
}

impl ScreenSystem {
    pub fn new(
        gfx_queue: Arc<Queue>,
        subpass: Subpass,
        color_view: Arc<ImageView<AttachmentImage>>,
        viewport: &Viewport,
    ) -> Result<Self, Error> {
        let (vertex_buffer, init) = ImmutableBuffer::from_iter(
            vec![
                SimpleVertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                },
                SimpleVertex {
                    v_position: Point3::new(1.0, -1.0, 0.0),
                },
                SimpleVertex {
                    v_position: Point3::new(1.0, 1.0, 0.0),
                },
                SimpleVertex {
                    v_position: Point3::new(1.0, 1.0, 0.0),
                },
                SimpleVertex {
                    v_position: Point3::new(-1.0, 1.0, 0.0),
                },
                SimpleVertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                },
            ],
            BufferUsage::vertex_buffer(),
            gfx_queue.clone(),
        )?;

        init.then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        let vs = shader::screen_vs::load(gfx_queue.device().clone()).unwrap();
        let fs = shader::screen_fs::load(gfx_queue.device().clone()).unwrap();

        let pipeline = Self::create_screen_pipeline(
            gfx_queue.device().clone(),
            viewport.clone(),
            subpass.clone(),
            vs.clone(),
            fs.clone(),
        );

        let screen_layout = pipeline.layout().set_layouts().get(0).unwrap();

        let screen_set = PersistentDescriptorSet::new(
            screen_layout.clone(),
            vec![WriteDescriptorSet::image_view(0, color_view)],
        )?;

        Ok(Self {
            gfx_queue,
            subpass,
            vertex_buffer,
            screen_set,
            vs,
            fs,
            pipeline,
        })
    }

    pub fn do_frame(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> Result<(), Error> {
        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.screen_set.clone(),
            )
            .draw(6, 1, 0, 0)?;

        Ok(())
    }

    pub fn swapchain_invalidated(
        &mut self,
        viewport: &Viewport,
        color_view: Arc<ImageView<AttachmentImage>>,
    ) -> Result<(), Error> {
        self.pipeline = Self::create_screen_pipeline(
            self.gfx_queue.device().clone(),
            viewport.clone(),
            self.subpass.clone(),
            self.vs.clone(),
            self.fs.clone(),
        );

        let screen_layout = self.pipeline.layout().set_layouts().get(0).unwrap();

        self.screen_set = PersistentDescriptorSet::new(
            screen_layout.clone(),
            vec![WriteDescriptorSet::image_view(0, color_view)],
        )?;

        Ok(())
    }

    fn create_screen_pipeline(
        device: Arc<Device>,
        viewport: Viewport,
        subpass: Subpass,
        screen_vs: Arc<ShaderModule>,
        screen_fs: Arc<ShaderModule>,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<SimpleVertex>())
            .input_assembly_state(InputAssemblyState::new())
            .render_pass(subpass)
            .vertex_shader(screen_vs.entry_point("main").unwrap(), ())
            .fragment_shader(screen_fs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .build(device)
            .unwrap()
    }
}

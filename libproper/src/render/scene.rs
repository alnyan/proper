use std::sync::Arc;

use nalgebra::Point3;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    device::Queue,
    format::{ClearValue, Format},
    image::{view::ImageView, SwapchainImage},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    sync::GpuFuture,
};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use crate::{event::Event, layer::Layer};

use super::{frame::Frame, shader, Vertex};

pub struct SceneLayer {
    gfx_queue: Arc<Queue>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    render_pass: Arc<RenderPass>,

    // Dummy stuff
    mode: u32,
    triangle_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    quad_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
}

impl SceneLayer {
    pub fn new(
        gfx_queue: Arc<Queue>,
        output_format: Format,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
        viewport: Viewport,
    ) -> Self {
        let render_pass = vulkano::single_pass_renderpass!(
            gfx_queue.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: output_format,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        let vs = shader::scene_vs::load(gfx_queue.device().clone()).unwrap();
        let fs = shader::scene_fs::load(gfx_queue.device().clone()).unwrap();

        let pipeline = Self::create_pipeline(&gfx_queue, &vs, &fs, viewport, render_pass.clone());
        let framebuffers = Self::create_framebuffers(&render_pass, swapchain_images);

        let triangle_buffer = CpuAccessibleBuffer::from_iter(
            gfx_queue.device().clone(),
            BufferUsage::vertex_buffer(),
            false,
            vec![
                Vertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(0.0, 1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, -1.0, 0.0),
                },
            ],
        )
        .unwrap();

        let quad_buffer = CpuAccessibleBuffer::from_iter(
            gfx_queue.device().clone(),
            BufferUsage::vertex_buffer(),
            false,
            vec![
                Vertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, -1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, 1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(1.0, 1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(-1.0, 1.0, 0.0),
                },
                Vertex {
                    v_position: Point3::new(-1.0, -1.0, 0.0),
                },
            ],
        )
        .unwrap();

        Self {
            gfx_queue,
            render_pass,
            vs,
            fs,
            pipeline,
            framebuffers,

            mode: 0,
            triangle_buffer,
            quad_buffer,
        }
    }

    fn create_pipeline(
        gfx_queue: &Arc<Queue>,
        vs: &Arc<ShaderModule>,
        fs: &Arc<ShaderModule>,
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .input_assembly_state(InputAssemblyState::new())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(gfx_queue.device().clone())
            .unwrap()
    }

    fn create_framebuffers(
        render_pass: &Arc<RenderPass>,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) -> Vec<Arc<Framebuffer>> {
        swapchain_images
            .into_iter()
            .map(|image| {
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![image.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect()
    }
}

impl Layer for SceneLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_event(&mut self, event: &Event, _: &mut ControlFlow) -> bool {
        if let Event::SwapchainInvalidated(images, viewport) = event {
            self.framebuffers = Self::create_framebuffers(&self.render_pass, images);
            self.pipeline = Self::create_pipeline(
                &self.gfx_queue,
                &self.vs,
                &self.fs,
                viewport.clone(),
                self.render_pass.clone(),
            );
        }

        // TODO a way to send/receive events from other layers
        if let Event::WindowEventWrapped(WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        }) = event
        {
            self.mode = 1 - self.mode;
        }

        false
    }

    fn on_draw(&mut self, in_future: Box<dyn GpuFuture>, frame: &Frame) -> Box<dyn GpuFuture> {
        let mut builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let framebuffer = self.framebuffers[frame.image_index].clone();

        let mut render_pass_begin_info = RenderPassBeginInfo::framebuffer(framebuffer);

        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])));

        let buffer;
        if self.mode == 0 {
            buffer = &self.triangle_buffer;
        } else {
            buffer = &self.quad_buffer;
        }

        builder
            .begin_render_pass(render_pass_begin_info, SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, buffer.clone())
            .draw(buffer.len().try_into().unwrap(), 1, 0, 0)
            .unwrap()
            .end_render_pass()
            .unwrap();

        let cb = builder.build().unwrap();

        in_future
            .then_execute(self.gfx_queue.clone(), cb)
            .unwrap()
            .boxed()
    }
}

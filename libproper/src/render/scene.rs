use std::{sync::Arc, time::Instant};

use nalgebra::{Matrix4, Point3, Vector3};
use vulkano::{
    buffer::{BufferUsage, CpuBufferPool, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::{ClearValue, Format},
    image::{view::ImageView, SwapchainImage},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    sync::GpuFuture,
};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ControlFlow,
    window::Window, dpi::PhysicalSize,
};

use crate::{
    event::Event,
    layer::Layer,
    resource::model::Model,
    world::{
        entity::Entity,
        scene::{MeshObject, Scene},
    },
};

use super::{frame::Frame, shader, Vertex};

pub struct SceneLayer {
    gfx_queue: Arc<Queue>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    render_pass: Arc<RenderPass>,
    scene_pool: CpuBufferPool<shader::scene_vs::ty::Scene_Data>,
    start_time: Instant,
    dimensions: (f32, f32),

    // Dummy stuff
    scene: Scene,
}

impl SceneLayer {
    pub fn new(
        gfx_queue: Arc<Queue>,
        output_format: Format,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
        viewport: Viewport,
        dimensions: PhysicalSize<u32>,
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

        let mut scene = Scene::default();

        let triangle_model = Arc::new(Model::triangle(gfx_queue.clone()));
        let cube_model = Arc::new(Model::cube(gfx_queue.clone()));

        let mesh0 = MeshObject::new(gfx_queue.clone(), cube_model);
        scene
            .entities
            .push(Entity::new(Point3::new(0.0, 0.0, 0.0), Some(mesh0)));

        let mesh1 = MeshObject::new(gfx_queue.clone(), triangle_model);
        scene
            .entities
            .push(Entity::new(Point3::new(2.0, 0.0, 0.0), Some(mesh1)));

        let scene_pool =
            CpuBufferPool::new(gfx_queue.device().clone(), BufferUsage::uniform_buffer());

        let start_time = Instant::now();

        let dimensions = dimensions.into();

        Self {
            gfx_queue,
            render_pass,
            vs,
            fs,
            pipeline,
            framebuffers,
            scene_pool,
            start_time,
            dimensions,

            scene,
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
        if let Event::SwapchainInvalidated { swapchain_images, viewport, dimensions } = event {
            self.framebuffers = Self::create_framebuffers(&self.render_pass, swapchain_images);
            self.pipeline = Self::create_pipeline(
                &self.gfx_queue,
                &self.vs,
                &self.fs,
                viewport.clone(),
                self.render_pass.clone(),
            );
            self.dimensions = dimensions.clone().into();
        }

        // TODO a way to send/receive events from other layers
        if let Event::WindowEventWrapped(WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        }) = event
        {
            todo!()
        }

        false
    }

    fn on_draw(&mut self, in_future: Box<dyn GpuFuture>, frame: &Frame) -> Box<dyn GpuFuture> {
        let scene_subbuffer = {
            let now = Instant::now();
            let t = (now - self.start_time).as_secs_f64();

            let camera_position = Point3::new(t.cos() as f32 * 5.0, 5.0, t.sin() as f32 * 5.0);
            let view = Matrix4::look_at_rh(
                &camera_position,
                &Point3::new(0.0, 0.0, 0.0),
                &Vector3::new(0.0, 1.0, 0.0),
            );
            let projection =
                Matrix4::new_perspective(self.dimensions.0 / self.dimensions.1, 45.0, 0.01, 100.0);

            let data = shader::scene_vs::ty::Scene_Data {
                projection: projection.into(),
                view: view.into(),
            };

            self.scene_pool.next(data).unwrap()
        };

        let scene_layout = self.pipeline.layout().set_layouts().get(0).unwrap();
        let model_layout = self.pipeline.layout().set_layouts().get(1).unwrap();

        let scene_set = PersistentDescriptorSet::new(
            scene_layout.clone(),
            vec![WriteDescriptorSet::buffer(0, scene_subbuffer)],
        )
        .unwrap();

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

        builder
            .begin_render_pass(render_pass_begin_info, SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                scene_set,
            );

        for object in self.scene.iter() {
            if let Some(mesh) = object.mesh() {
                let model_set = PersistentDescriptorSet::new(
                    model_layout.clone(),
                    vec![WriteDescriptorSet::buffer(0, mesh.model_buffer().clone())],
                )
                .unwrap();

                let model = mesh.model();
                let model_data = model.data().unwrap();

                builder
                    .bind_vertex_buffers(0, model_data.clone())
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.pipeline.layout().clone(),
                        1,
                        model_set,
                    )
                    .draw(model_data.len().try_into().unwrap(), 1, 0, 0)
                    .unwrap();
            }
        }

        builder.end_render_pass().unwrap();

        let cb = builder.build().unwrap();

        in_future
            .then_execute(self.gfx_queue.clone(), cb)
            .unwrap()
            .boxed()
    }
}

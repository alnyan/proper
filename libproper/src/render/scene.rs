use std::{sync::Arc, time::Instant};

use nalgebra::{Matrix4, Point3, Vector3};
use vulkano::{
    buffer::{BufferUsage, CpuBufferPool, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{
        layout::{DescriptorSetLayout, DescriptorSetLayoutCreateInfo},
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    format::{ClearValue, Format},
    image::{view::ImageView, AttachmentImage, ImageViewAbstract, SwapchainImage},
    pipeline::{
        graphics::viewport::Viewport, layout::PipelineLayoutCreateInfo, Pipeline,
        PipelineBindPoint, PipelineLayout,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    sync::GpuFuture,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use crate::{
    event::Event,
    layer::Layer,
    resource::{
        material::{MaterialInstanceCreateInfo, MaterialRegistry, SimpleMaterial},
        model::Model,
    },
    world::{
        entity::Entity,
        scene::{MeshObject, Scene},
    },
};

use super::{frame::Frame, shader};

pub struct SceneLayer {
    gfx_queue: Arc<Queue>,
    framebuffers: Vec<Arc<Framebuffer>>,
    depth_view: Arc<ImageView<AttachmentImage>>,
    render_pass: Arc<RenderPass>,
    scene_pool: CpuBufferPool<shader::simple_vs::ty::Scene_Data>,
    start_time: Instant,
    dimensions: (f32, f32),

    // Dummy stuff
    common_pipeline_layout: Arc<PipelineLayout>,
    scene: Scene,
    material_registry: MaterialRegistry,
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
        )
        .unwrap();

        let (framebuffers, depth_view) =
            Self::create_framebuffers(gfx_queue.device().clone(), &render_pass, swapchain_images);

        let mut material_registry = MaterialRegistry::new();

        let mat_simple_id = material_registry.add(
            "simple",
            Box::new(SimpleMaterial::new(&gfx_queue, &render_pass, &viewport)),
        );

        let mut scene = Scene::default();

        let triangle_model = Arc::new(Model::triangle(gfx_queue.clone(), mat_simple_id));
        let cube_model = Arc::new(Model::cube(gfx_queue.clone(), mat_simple_id));

        const SIZE: i32 = 4;
        for x in -SIZE..=SIZE {
            for y in -SIZE..=SIZE {
                let create_info = MaterialInstanceCreateInfo::default().with_color(
                    "diffuse_color",
                    [
                        (x + SIZE) as f32 / (SIZE * 2 + 1) as f32,
                        0.0,
                        (y + SIZE) as f32 / (SIZE * 2 + 1) as f32,
                        1.0,
                    ],
                );

                let mesh = if (x + y) % 2 == 0 {
                    MeshObject::new(
                        gfx_queue.clone(),
                        triangle_model.clone(),
                        &material_registry,
                        create_info.clone(),
                    )
                } else {
                    MeshObject::new(
                        gfx_queue.clone(),
                        cube_model.clone(),
                        &material_registry,
                        create_info.clone(),
                    )
                };

                let entity = Entity::new(Point3::new(x as f32, 0.0, y as f32), Some(mesh));

                scene.add(entity);
            }
        }

        let scene_pool =
            CpuBufferPool::new(gfx_queue.device().clone(), BufferUsage::uniform_buffer());

        let start_time = Instant::now();

        let dimensions = dimensions.into();

        // Have to load these in order to access DescriptorRequirements
        let dummy_vs = shader::simple_vs::load(gfx_queue.device().clone()).unwrap();
        let dummy_vs_entry = dummy_vs.entry_point("main").unwrap();
        let dummy_fs = shader::simple_fs::load(gfx_queue.device().clone()).unwrap();
        let dummy_fs_entry = dummy_fs.entry_point("main").unwrap();

        let descriptor_set_layout_create_infos = DescriptorSetLayoutCreateInfo::from_requirements(
            dummy_vs_entry
                .descriptor_requirements()
                .filter(|((set, _), _)| *set != 1)
                .chain(
                    dummy_fs_entry
                        .descriptor_requirements()
                        .filter(|((set, _), _)| *set != 1),
                ),
        );
        let descriptor_set_layouts = descriptor_set_layout_create_infos
            .into_iter()
            .map(|info| DescriptorSetLayout::new(gfx_queue.device().clone(), info).unwrap())
            .collect();
        let common_pipeline_layout = PipelineLayout::new(
            gfx_queue.device().clone(),
            PipelineLayoutCreateInfo {
                set_layouts: descriptor_set_layouts,
                push_constant_ranges: vec![],
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            gfx_queue,
            render_pass,
            framebuffers,
            depth_view,
            scene_pool,
            start_time,
            dimensions,

            material_registry,
            scene,
            common_pipeline_layout,
        }
    }

    fn create_framebuffers(
        device: Arc<Device>,
        render_pass: &Arc<RenderPass>,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) -> (Vec<Arc<Framebuffer>>, Arc<ImageView<AttachmentImage>>) {
        let depth_view = ImageView::new_default(
            AttachmentImage::transient(
                device,
                swapchain_images[0].dimensions().width_height(),
                Format::D16_UNORM,
            )
            .unwrap(),
        )
        .unwrap();

        (
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
                    .unwrap()
                })
                .collect(),
            depth_view,
        )
    }
}

impl Layer for SceneLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_event(&mut self, event: &Event, _: &mut ControlFlow) -> bool {
        if let Event::SwapchainInvalidated {
            swapchain_images,
            viewport,
            dimensions,
        } = event
        {
            (self.framebuffers, self.depth_view) = Self::create_framebuffers(
                self.gfx_queue.device().clone(),
                &self.render_pass,
                swapchain_images,
            );
            self.dimensions = (*dimensions).into();

            self.material_registry
                .recreate_pipelines(&self.gfx_queue, &self.render_pass, viewport);
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
        let t0 = Instant::now();
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

            // TODO use some common data type for this
            let data = shader::simple_vs::ty::Scene_Data {
                projection: projection.into(),
                view: view.into(),
            };

            self.scene_pool.next(data).unwrap()
        };

        let scene_layout = self.common_pipeline_layout.set_layouts().get(0).unwrap();
        let model_layout = self.common_pipeline_layout.set_layouts().get(2).unwrap();

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
        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Depth(1.0)));

        builder
            .begin_render_pass(render_pass_begin_info, SubpassContents::Inline)
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.common_pipeline_layout.clone(),
                0,
                scene_set,
            );

        for group in self.scene.iter() {
            let material_template = self.material_registry.get(group.material_template_id());
            let pipeline = material_template.pipeline();

            // Bind template
            builder.bind_pipeline_graphics(pipeline.clone());

            for object in group.iter() {
                if let Some(mesh) = object.mesh() {
                    let model_set = PersistentDescriptorSet::new(
                        model_layout.clone(),
                        vec![WriteDescriptorSet::buffer(0, mesh.model_buffer().clone())],
                    )
                    .unwrap();

                    let model = mesh.model();
                    let model_data = model.data().unwrap();

                    mesh.material_instance().bind_data(&mut builder, pipeline);

                    builder
                        .bind_vertex_buffers(0, model_data.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            pipeline.layout().clone(),
                            2,
                            model_set,
                        )
                        .draw(model_data.len().try_into().unwrap(), 1, 0, 0)
                        .unwrap();
                }
            }
        }

        builder.end_render_pass().unwrap();

        let cb = builder.build().unwrap();

        let t1 = Instant::now();
        log::debug!("Command buffer build: {:?}", t1 - t0);

        in_future
            .then_execute(self.gfx_queue.clone(), cb)
            .unwrap()
            .boxed()
    }
}

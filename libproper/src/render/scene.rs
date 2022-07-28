use std::{sync::Arc, time::Instant};

use nalgebra::{Matrix4, Point3, Vector3};
use vulkano::{
    buffer::{BufferUsage, CpuBufferPool},
    descriptor_set::layout::{DescriptorSetLayout, DescriptorSetLayoutCreateInfo},
    device::{Device, Queue},
    format::Format,
    image::{view::ImageView, AttachmentImage, ImageViewAbstract, SampleCount, SwapchainImage},
    instance::InstanceCreationError,
    pipeline::{graphics::viewport::Viewport, layout::PipelineLayoutCreateInfo, PipelineLayout},
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
    error::Error,
    event::Event,
    layer::Layer,
    resource::{material::MaterialInstanceCreateInfo, model::Model},
    world::{
        entity::Entity,
        scene::{MeshObject, Scene},
    },
};

use super::{frame::Frame, shader, system::forward::ForwardSystem};

type FramebufferCreateOutput = (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
);

pub struct SceneLayer {
    gfx_queue: Arc<Queue>,
    scene_pool: CpuBufferPool<shader::simple_vs::ty::Scene_Data>,
    scene: Scene,

    render_pass: Arc<RenderPass>,

    framebuffers: Vec<Arc<Framebuffer>>,
    color_view: Arc<ImageView<AttachmentImage>>,
    depth_view: Arc<ImageView<AttachmentImage>>,

    forward_system: ForwardSystem,

    start_time: Instant,
    dimensions: (f32, f32),
}

impl SceneLayer {
    pub fn new(
        gfx_queue: Arc<Queue>,
        output_format: Format,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
        viewport: Viewport,
        dimensions: PhysicalSize<u32>,
    ) -> Result<Self, Error> {
        // Have to load these in order to access DescriptorRequirements
        let dummy_vs = shader::simple_vs::load(gfx_queue.device().clone())?;
        let dummy_vs_entry = dummy_vs
            .entry_point("main")
            .ok_or(Error::MissingShaderEntryPoint)?;
        let dummy_fs = shader::simple_fs::load(gfx_queue.device().clone())?;
        let dummy_fs_entry = dummy_fs
            .entry_point("main")
            .ok_or(Error::MissingShaderEntryPoint)?;

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
            .map(|info| {
                DescriptorSetLayout::new(gfx_queue.device().clone(), info).map_err(Error::from)
            })
            .collect::<Result<_, _>>()?;
        let common_pipeline_layout = PipelineLayout::new(
            gfx_queue.device().clone(),
            PipelineLayoutCreateInfo {
                set_layouts: descriptor_set_layouts,
                push_constant_ranges: vec![],
                ..Default::default()
            },
        )?;

        let render_pass = vulkano::ordered_passes_renderpass!(
            gfx_queue.device().clone(),
            attachments: {
                ms_color: {
                    load: Clear,
                    store: DontCare,
                    format: output_format,
                    samples: 4,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 4,
                },
                final_color: {
                    load: Clear,
                    store: Store,
                    format: output_format,
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [ms_color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: [ms_color]
                }
            ]
        )?;

        let forward_system = ForwardSystem::new(
            gfx_queue.clone(),
            viewport,
            render_pass.clone(),
            common_pipeline_layout.clone(),
        )?;

        let mut scene = Scene::default();

        let mat_simple_id = forward_system
            .material_registry()
            .lock()
            .unwrap()
            .get_id("simple")
            .unwrap();
        let model0 = Arc::new(Model::load_to_device(
            gfx_queue.clone(),
            "res/models/monkey.obj",
            mat_simple_id,
        )?);
        let model1 = Arc::new(Model::load_to_device(
            gfx_queue.clone(),
            "res/models/torus.obj",
            mat_simple_id,
        )?);
        // let cube_model = Arc::new(Model::cube(gfx_queue.clone(), mat_simple_id)?);

        const SIZE: i32 = 24;
        let mut lock = forward_system.material_registry().lock().unwrap();
        for x in -SIZE..=SIZE {
            for y in -SIZE..=SIZE {
                let v = if (x + y) % 2 == 0 { 1.0 } else { 0.0 };

                let create_info = MaterialInstanceCreateInfo::default().with_color(
                    "diffuse_color",
                    [
                        (x + SIZE) as f32 / (SIZE * 2 + 1) as f32,
                        v,
                        (y + SIZE) as f32 / (SIZE * 2 + 1) as f32,
                        1.0,
                    ],
                );

                let mesh = if (x + y) % 2 == 0 {
                    MeshObject::new(
                        gfx_queue.clone(),
                        model0.clone(),
                        &mut lock,
                        create_info.clone(),
                    )?
                } else {
                    MeshObject::new(
                        gfx_queue.clone(),
                        model1.clone(),
                        &mut lock,
                        create_info.clone(),
                    )?
                };

                let entity = Entity::new(Point3::new(x as f32, v, y as f32), Some(mesh))?;

                scene.add(entity);
            }
        }
        drop(lock);

        let scene_pool =
            CpuBufferPool::new(gfx_queue.device().clone(), BufferUsage::uniform_buffer());

        let start_time = Instant::now();

        let dimensions = dimensions.into();

        let (framebuffers, color_view, depth_view) =
            Self::create_framebuffers(gfx_queue.device().clone(), &render_pass, swapchain_images)?;

        Ok(Self {
            gfx_queue,
            dimensions,
            scene_pool,

            framebuffers,
            color_view,
            depth_view,

            render_pass,

            forward_system,

            scene,
            start_time,
        })
    }

    fn create_framebuffers(
        device: Arc<Device>,
        render_pass: &Arc<RenderPass>,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) -> Result<FramebufferCreateOutput, Error> {
        let color_view = ImageView::new_default(
            AttachmentImage::transient_multisampled_input_attachment(
                device.clone(),
                swapchain_images[0].dimensions().width_height(),
                SampleCount::Sample4,
                swapchain_images[0].format().unwrap(),
            )
            .unwrap(),
        )?;
        let depth_view = ImageView::new_default(AttachmentImage::transient_multisampled(
            device,
            swapchain_images[0].dimensions().width_height(),
            SampleCount::Sample4,
            Format::D16_UNORM,
        )?)?;

        Ok((
            swapchain_images
                .into_iter()
                .enumerate()
                .map(|(i, image)| {
                    Framebuffer::new(
                        render_pass.clone(),
                        FramebufferCreateInfo {
                            attachments: vec![
                                color_view.clone(),
                                depth_view.clone(),
                                image.clone(),
                            ],
                            ..Default::default()
                        },
                    )
                })
                .collect::<Result<_, _>>()
                .map_err(Error::from)?,
            color_view,
            depth_view,
        ))
    }
}

impl Layer for SceneLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_event(&mut self, event: &Event, _: &mut ControlFlow) -> Result<bool, Error> {
        if let Event::SwapchainInvalidated {
            swapchain_images,
            viewport,
            dimensions,
        } = event
        {
            self.dimensions = (*dimensions).into();
            (self.framebuffers, self.color_view, self.depth_view) = Self::create_framebuffers(
                self.gfx_queue.device().clone(),
                &self.render_pass,
                swapchain_images,
            )?;

            self.forward_system
                .swapchain_invalidated(viewport, swapchain_images)?;
            return Ok(false);
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

        Ok(false)
    }

    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error> {
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

            self.scene_pool.next(data)?
        };

        let forward_cb = self.forward_system.do_frame(
            scene_subbuffer,
            self.color_view.clone(),
            self.framebuffers[frame.image_index].clone(),
            frame,
            &self.scene,
        )?;

        Ok(in_future
            .then_execute(self.gfx_queue.clone(), forward_cb)?
            .boxed())
    }
}

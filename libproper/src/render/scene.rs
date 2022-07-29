use std::{path::Path, sync::Arc, time::Instant};

use nalgebra::{Matrix4, Point3, Vector3};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{
        layout::{DescriptorSetLayout, DescriptorSetLayoutCreateInfo},
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    format::{ClearValue, Format},
    image::{
        view::ImageView, AttachmentImage, ImageDimensions, ImageViewAbstract, ImmutableImage,
        MipmapsCount, SampleCount, SwapchainImage,
    },
    pipeline::{graphics::viewport::Viewport, layout::PipelineLayoutCreateInfo, PipelineLayout},
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
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
    event::{Event, GameEvent},
    layer::Layer,
    resource::{material::MaterialInstanceCreateInfo, model::Model},
    world::{
        entity::Entity,
        scene::{MeshObject, Scene},
    },
};

use super::{
    frame::Frame,
    shader,
    system::{forward::ForwardSystem, screen::ScreenSystem},
};

type FramebufferCreateOutput = (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
);

pub struct SceneLayer {
    gfx_queue: Arc<Queue>,
    scene: Scene,
    scene_buffer: Arc<CpuAccessibleBuffer<shader::simple_vs::ty::Scene_Data>>,
    scene_set: Arc<PersistentDescriptorSet>,

    render_pass: Arc<RenderPass>,

    // TODO move this to some ModelRegistry
    sampler: Arc<Sampler>,
    texture0: Arc<ImageView<ImmutableImage>>,
    texture1: Arc<ImageView<ImmutableImage>>,
    model0: Arc<Model>,
    model1: Arc<Model>,

    framebuffers: Vec<Arc<Framebuffer>>,
    color_view: Arc<ImageView<AttachmentImage>>,
    depth_view: Arc<ImageView<AttachmentImage>>,

    forward_system: ForwardSystem,
    screen_system: ScreenSystem,

    start_time: Instant,
    dimensions: (f32, f32),
}

fn load_texture<P: AsRef<Path>>(gfx_queue: Arc<Queue>, path: P) -> Arc<ImageView<ImmutableImage>> {
    let image = image::open(path).unwrap();
    let width = image.width();
    let height = image.height();
    let data = image.into_rgba8();

    let (texture, init) = ImmutableImage::from_iter(
        data.into_raw(),
        ImageDimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        },
        MipmapsCount::One,
        Format::R8G8B8A8_UNORM,
        gfx_queue,
    )
    .unwrap();

    init.then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    ImageView::new_default(texture).unwrap()
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

        let (framebuffers, color_view, depth_view) =
            Self::create_framebuffers(gfx_queue.device().clone(), &render_pass, swapchain_images)?;

        let forward_system = ForwardSystem::new(
            gfx_queue.clone(),
            &viewport,
            render_pass.clone(),
            common_pipeline_layout.clone(),
        )?;

        let screen_system = ScreenSystem::new(
            gfx_queue.clone(),
            render_pass.clone(),
            color_view.clone(),
            &viewport,
        )?;

        let scene = Scene::default();

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
        let texture0 = load_texture(gfx_queue.clone(), "res/textures/texture0.png");
        let texture1 = load_texture(gfx_queue.clone(), "res/textures/texture1.png");
        let sampler = Sampler::new(
            gfx_queue.device().clone(),
            SamplerCreateInfo {
                address_mode: [SamplerAddressMode::Repeat; 3],
                min_filter: Filter::Nearest,
                mag_filter: Filter::Nearest,
                ..Default::default()
            },
        )
        .unwrap();

        let start_time = Instant::now();

        let scene_buffer = unsafe {
            CpuAccessibleBuffer::uninitialized(
                gfx_queue.device().clone(),
                BufferUsage::uniform_buffer(),
                false,
            )?
        };

        let scene_layout = common_pipeline_layout.set_layouts().get(0).unwrap();
        let scene_set = PersistentDescriptorSet::new(
            scene_layout.clone(),
            vec![WriteDescriptorSet::buffer(0, scene_buffer.clone())],
        )?;

        let dimensions = dimensions.into();

        Ok(Self {
            gfx_queue,
            dimensions,
            scene_buffer,
            scene_set,

            model0,
            model1,
            texture0,
            texture1,
            sampler,

            framebuffers,
            color_view,
            depth_view,

            render_pass,

            forward_system,
            screen_system,

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
                .map(|image| {
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

            self.forward_system.swapchain_invalidated(viewport)?;
            self.screen_system
                .swapchain_invalidated(viewport, self.color_view.clone())?;
            return Ok(false);
        }

        // Click on the scene, TODO
        if let Event::WindowEventWrapped(WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        }) = event
        {
            todo!()
        }

        // Events from other layers
        if let Event::GameEvent(GameEvent::TestEvent) = event {
            let mut lock = self.forward_system.material_registry().lock().unwrap();

            let b = rand::random();
            let t = rand::random();

            let x = rand::random();
            let y = rand::random();
            let z = rand::random();
            let position = Point3::new((x - 0.5) * 5.0, y, (z - 0.5) * 5.0);

            let model = if b {
                self.model0.clone()
            } else {
                self.model1.clone()
            };
            let texture = if t {
                self.texture0.clone()
            } else {
                self.texture1.clone()
            };

            let create_info = MaterialInstanceCreateInfo::default()
                .with_color("diffuse_color", [x, y, z, 1.0])
                .with_texture("diffuse_map", self.sampler.clone(), texture);

            let mesh = MeshObject::new(
                self.gfx_queue.clone(),
                model,
                &mut lock,
                create_info.clone(),
            )?;

            let entity = Entity::new(position, Some(mesh))?;

            self.scene.add(entity);
        }

        Ok(false)
    }

    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error> {
        {
            let mut data = self.scene_buffer.write()?;

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
            *data = shader::simple_vs::ty::Scene_Data {
                projection: projection.into(),
                view: view.into(),
            };
        };

        let framebuffer = &self.framebuffers[frame.image_index];

        let mut builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let mut render_pass_begin_info = RenderPassBeginInfo::framebuffer(framebuffer.clone());

        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])));
        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Depth(1.0)));
        render_pass_begin_info
            .clear_values
            .push(Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])));

        builder.begin_render_pass(
            render_pass_begin_info,
            SubpassContents::SecondaryCommandBuffers,
        )?;

        self.forward_system
            .do_frame(&mut builder, &self.scene_set, &self.scene)?;

        builder.next_subpass(SubpassContents::Inline)?;

        self.screen_system.do_frame(&mut builder)?;

        builder.end_render_pass()?;

        let cb = builder.build()?;

        Ok(in_future.then_execute(self.gfx_queue.clone(), cb)?.boxed())
    }
}

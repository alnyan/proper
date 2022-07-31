use std::sync::{Arc, Mutex};

use nalgebra::{Matrix4, Vector3};
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
    image::{view::ImageView, AttachmentImage, ImageViewAbstract, SampleCount, SwapchainImage},
    pipeline::{graphics::viewport::Viewport, layout::PipelineLayoutCreateInfo, PipelineLayout},
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};
use winit::{dpi::PhysicalSize, event_loop::ControlFlow, window::Window};

use crate::{
    error::Error,
    event::Event,
    layer::Layer,
    render::{
        frame::Frame,
        shader,
        system::{forward::ForwardSystem, screen::ScreenSystem},
    },
    resource::material::MaterialRegistry,
    world::scene::Scene,
};

type FramebufferCreateOutput = (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
);

pub struct WorldLayer {
    gfx_queue: Arc<Queue>,
    scene: Arc<Mutex<Scene>>,
    scene_buffer: Arc<CpuAccessibleBuffer<shader::simple_vs::ty::Scene_Data>>,
    scene_set: Arc<PersistentDescriptorSet>,

    material_registry: Arc<Mutex<MaterialRegistry>>,
    render_pass: Arc<RenderPass>,

    framebuffers: Vec<Arc<Framebuffer>>,
    color_view: Arc<ImageView<AttachmentImage>>,
    depth_view: Arc<ImageView<AttachmentImage>>,

    forward_system: ForwardSystem,
    screen_system: ScreenSystem,

    dimensions: (f32, f32),
}

impl WorldLayer {
    pub fn new(
        gfx_queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        material_registry: Arc<Mutex<MaterialRegistry>>,
        swapchain_images: &Vec<Arc<ImageView<SwapchainImage<Window>>>>,
        viewport: Viewport,
        dimensions: PhysicalSize<u32>,
        scene: Arc<Mutex<Scene>>,
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

        let (framebuffers, color_view, depth_view) =
            Self::create_framebuffers(gfx_queue.device().clone(), &render_pass, swapchain_images)?;

        let forward_system = ForwardSystem::new(
            gfx_queue.clone(),
            Subpass::from(render_pass.clone(), 0).unwrap(),
            common_pipeline_layout.clone(),
        )?;

        let screen_system = ScreenSystem::new(
            gfx_queue.clone(),
            Subpass::from(render_pass.clone(), 1).unwrap(),
            color_view.clone(),
            &viewport,
        )?;

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

            framebuffers,
            color_view,
            depth_view,

            material_registry,
            render_pass,

            forward_system,
            screen_system,

            scene,
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

impl Layer for WorldLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_tick(&mut self, _delta: f64) -> Result<(), Error> {
        Ok(())
    }

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

            self.material_registry
                .lock()
                .unwrap()
                .recreate_pipelines(viewport)?;
            self.screen_system
                .swapchain_invalidated(viewport, self.color_view.clone())?;
            return Ok(false);
        }

        Ok(false)
    }

    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error> {
        let scene_lock = self.scene.lock().unwrap();

        {
            let mut data = self.scene_buffer.write()?;

            let view = Matrix4::look_at_rh(
                scene_lock.camera.position(),
                &(scene_lock.camera.position() + scene_lock.camera.forward()),
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
            .do_frame(&mut builder, &self.scene_set, scene_lock)?;

        builder.next_subpass(SubpassContents::Inline)?;

        self.screen_system.do_frame(&mut builder)?;

        builder.end_render_pass()?;

        let cb = builder.build()?;

        Ok(in_future.then_execute(self.gfx_queue.clone(), cb)?.boxed())
    }
}

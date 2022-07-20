use std::sync::{Arc, Mutex};

use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageUsage, ImageViewAbstract, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::graphics::viewport::Viewport,
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder}, dpi::PhysicalSize,
};

use crate::{event::Event, layer::Layer};

use super::frame::Frame;

pub type LayerVec = Arc<Mutex<Vec<Box<dyn Layer>>>>;

pub struct VulkanContext {
    surface: Arc<Surface<Window>>,

    device: Arc<Device>,
    queue: Arc<Queue>,

    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    viewport: Viewport,
    need_swapchain_recreation: bool,

    layers: LayerVec,
}

impl VulkanContext {
    pub fn new_windowed(
        event_loop: &EventLoop<()>,
        window_builder: WindowBuilder,
        layers: LayerVec,
    ) -> Self {
        log::debug!("Creating new windowed vulkan context");

        let instance_extensions = vulkano_win::required_extensions();
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_maintenance1: true,
            ..DeviceExtensions::none()
        };

        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: instance_extensions,
            ..Default::default()
        })
        .unwrap();

        let surface = window_builder
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let (physical, queue_family) = Self::select_physical_device(&instance, &surface);

        let (device, mut queues) = Device::new(
            physical,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                enabled_extensions: physical
                    .supported_extensions()
                    .intersection(&device_extensions),
                ..Default::default()
            },
        )
        .unwrap();
        let queue = queues.next().unwrap();

        let (swapchain, swapchain_images) = Self::create_swapchain(device.clone(), surface.clone());

        let viewport = Self::create_viewport(&surface);

        log::debug!("Vulkan init finished");

        Self {
            surface,
            device,
            queue,
            swapchain,
            swapchain_images,
            viewport,
            layers,
            need_swapchain_recreation: false,
        }
    }

    pub const fn gfx_queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    pub const fn surface(&self) -> &Arc<Surface<Window>> {
        &self.surface
    }

    pub const fn swapchain_images(&self) -> &Vec<Arc<ImageView<SwapchainImage<Window>>>> {
        &self.swapchain_images
    }

    pub const fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn output_format(&self) -> Format {
        self.swapchain_images[0].format().unwrap()
    }

    pub fn invalidate_surface(&mut self) {
        self.need_swapchain_recreation = true;
    }

    pub fn do_frame(&mut self, flow: &mut ControlFlow) {
        if self.need_swapchain_recreation {
            self.recreate_swapchain();

            // TODO use some "event dispatcher" for that
            let event = Event::SwapchainInvalidated(&self.swapchain_images, self.viewport.clone());
            for layer in self.layers.lock().unwrap().iter_mut() {
                // Ignore hierarchy, this event needs to be delivered to every layer
                layer.on_event(&event, flow);
            }
        }

        let (image_index, suboptimal, acquire_future) =
            swapchain::acquire_next_image(self.swapchain.clone(), None).unwrap();

        if suboptimal {
            self.need_swapchain_recreation = true;
        }

        let mut in_future: Box<dyn GpuFuture + 'static> = Box::new(acquire_future);
        let frame = Frame {
            image_index,
            gfx_queue: self.queue.clone(),
            destination: self.swapchain_images[image_index].clone(),
            viewport: self.viewport.clone(),
        };
        for layer in self.layers.lock().unwrap().iter_mut() {
            in_future = layer.on_draw(in_future, &frame);
        }

        let future = sync::now(self.device.clone())
            .join(in_future)
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_index)
            .then_signal_fence_and_flush()
            .unwrap();

        future.wait(None).unwrap();
    }

    fn recreate_swapchain(&mut self) -> PhysicalSize<u32> {
        let new_dimensions = self.surface.window().inner_size();
        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: new_dimensions.into(),
                ..self.swapchain.create_info()
            })
            .unwrap();

        self.swapchain = new_swapchain;
        self.swapchain_images = new_images
            .into_iter()
            .map(|image| ImageView::new_default(image).unwrap())
            .collect();

        self.viewport = Self::create_viewport(&self.surface);

        new_dimensions
    }

    fn select_physical_device<'b>(
        instance: &'b Arc<Instance>,
        surface: &Arc<Surface<Window>>,
    ) -> (PhysicalDevice<'b>, QueueFamily<'b>) {
        PhysicalDevice::enumerate(instance)
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| {
                        q.supports_graphics() && q.supports_surface(surface).unwrap_or(false)
                    })
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .unwrap()
    }

    fn create_swapchain(
        device: Arc<Device>,
        surface: Arc<Surface<Window>>,
    ) -> (
        Arc<Swapchain<Window>>,
        Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    ) {
        let caps = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();
        let image_format = Some(
            device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let (swapchain, images) = Swapchain::new(
            device,
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count,
                image_extent: surface.window().inner_size().into(),
                image_usage: ImageUsage::color_attachment(),
                composite_alpha: caps.supported_composite_alpha.iter().next().unwrap(),
                image_format,
                ..Default::default()
            },
        )
        .unwrap();

        (
            swapchain,
            images
                .into_iter()
                .map(|image| ImageView::new_default(image).unwrap())
                .collect(),
        )
    }

    fn create_viewport(surface: &Arc<Surface<Window>>) -> Viewport {
        let dim = surface.window().inner_size();
        Viewport {
            origin: [0.0, dim.height as f32],
            dimensions: [dim.width as f32, -(dim.height as f32)],
            depth_range: 0.0..1.0,
        }
    }
}

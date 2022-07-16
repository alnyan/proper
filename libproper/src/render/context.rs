use std::sync::Arc;

use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    image::{ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct VulkanContext {
    instance: Arc<Instance>,

    surface: Arc<Surface<Window>>,

    device: Arc<Device>,
    queue: Arc<Queue>,

    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

    recreate_swapchain: bool,
}

impl VulkanContext {
    pub fn new_windowed(event_loop: &EventLoop<()>, window_builder: WindowBuilder) -> Self {
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
                enabled_extensions: physical.required_extensions().union(&device_extensions),
                ..Default::default()
            },
        )
        .unwrap();
        let queue = queues.next().unwrap();

        let (swapchain, swapchain_images) = Self::create_swapchain(device.clone(), surface.clone());

        log::debug!("Vulkan init finished");

        Self {
            instance,
            surface,
            device,
            queue,
            swapchain,
            swapchain_images,
            recreate_swapchain: false,
        }
    }

    pub fn do_frame(&mut self) {
        if self.recreate_swapchain {
            todo!()
        }

        let (image_index, _, acquire_future) =
            swapchain::acquire_next_image(self.swapchain.clone(), None).unwrap();

        let future = sync::now(self.device.clone())
            .join(acquire_future)
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_index)
            .then_signal_fence_and_flush()
            .unwrap();

        future.wait(None).unwrap();
    }

    fn select_physical_device<'a>(
        instance: &'a Arc<Instance>,
        surface: &Arc<Surface<Window>>,
    ) -> (PhysicalDevice<'a>, QueueFamily<'a>) {
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
    ) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
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

        Swapchain::new(
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
        .unwrap()
    }
}

use std::sync::Arc;

use vulkano::{device::Queue, swapchain::Swapchain, render_pass::Framebuffer, image::{view::ImageView, SwapchainImage}};
use winit::window::Window;

pub struct Frame {
    pub gfx_queue: Arc<Queue>,
    pub image_index: usize,
    pub destination: Arc<ImageView<SwapchainImage<Window>>>
}

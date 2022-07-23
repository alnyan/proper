use std::sync::Arc;

use vulkano::{
    device::Queue,
    image::{view::ImageView, SwapchainImage},
    pipeline::graphics::viewport::Viewport,
};
use winit::window::Window;

pub struct Frame {
    pub gfx_queue: Arc<Queue>,
    pub image_index: usize,
    pub destination: Arc<ImageView<SwapchainImage<Window>>>,
    pub viewport: Viewport,
}

use std::sync::Arc;

use vulkano::{
    image::{view::ImageView, SwapchainImage},
    pipeline::graphics::viewport::Viewport,
};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

pub enum Event<'a> {
    SwapchainInvalidated(
        &'a Vec<Arc<ImageView<SwapchainImage<Window>>>>,
        Viewport,
    ),
    WindowResized(PhysicalSize<u32>),
    WindowCloseRequested,
    // Required for egui-winit compat
    WindowEventWrapped(&'a WindowEvent<'a>),
}

impl<'a> TryFrom<&'a WindowEvent<'a>> for Event<'a> {
    type Error = ();

    fn try_from(value: &'a WindowEvent<'a>) -> Result<Self, Self::Error> {
        match value {
            WindowEvent::Resized(new_size) => Ok(Self::WindowResized(*new_size)),
            WindowEvent::CloseRequested => Ok(Self::WindowCloseRequested),
            WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::KeyboardInput { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::Focused(_)
            | WindowEvent::ModifiersChanged(_)
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::ReceivedCharacter(_) => Ok(Self::WindowEventWrapped(value)),
            _ => Err(()),
        }
    }
}

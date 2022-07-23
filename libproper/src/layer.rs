use vulkano::sync::GpuFuture;
use winit::event_loop::ControlFlow;

use crate::{error::Error, event::Event, render::frame::Frame};

pub trait Layer {
    fn on_attach(&mut self);
    fn on_detach(&mut self);
    fn on_event(&mut self, event: &Event, flow: &mut ControlFlow) -> Result<bool, Error>;
    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error>;
}

use std::sync::{Mutex, MutexGuard};

use vulkano::sync::GpuFuture;
use winit::event_loop::ControlFlow;

use crate::{event::Event, render::{context::VulkanContext, frame::Frame}};


pub trait Layer {
    fn on_attach(&mut self);
    fn on_detach(&mut self);
    fn on_event(&mut self, event: &Event, flow: &mut ControlFlow) -> bool;
    fn on_draw(&mut self, in_future: Box<dyn GpuFuture>, frame: &Frame) -> Box<dyn GpuFuture>;
}

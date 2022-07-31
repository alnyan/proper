use vulkano::sync::GpuFuture;
use winit::event_loop::ControlFlow;

use crate::{error::Error, event::Event, render::frame::Frame};

pub mod world;
pub mod logic;
pub mod gui;

#[derive(Default)]
pub struct LayerManager {
    layers: Vec<Box<dyn Layer>>
}

pub trait Layer {
    fn on_attach(&mut self);
    fn on_detach(&mut self);
    fn on_event(&mut self, event: &Event, flow: &mut ControlFlow) -> Result<bool, Error>;
    fn on_tick(&mut self, delta: f64) -> Result<(), Error>;
    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error>;
}

impl LayerManager {
    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Layer>> {
        self.layers.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Layer>> {
        self.layers.iter_mut()
    }

    pub fn tick(&mut self, delta: f64) -> Result<(), Error> {
        for layer in self.layers.iter_mut() {
            layer.on_tick(delta).unwrap();
        }
        Ok(())
    }

    pub fn notify_all(&mut self, event: &Event, flow: &mut ControlFlow) -> Result<(), Error> {
        for layer in self.layers.iter_mut().rev() {
            if layer.on_event(event, flow)? {
                break;
            }
        }
        Ok(())
    }

    pub fn push(&mut self, layer: Box<dyn Layer>) {
        self.layers.push(layer);
    }
}

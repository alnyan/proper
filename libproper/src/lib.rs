#![allow(clippy::into_iter_on_ref)]

use std::sync::{Arc, Mutex};

use event::Event;
use render::{context::{VulkanContext, LayerVec}, gui::GuiLayer, scene::SceneLayer};
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod event;
pub mod layer;
pub mod render;
pub mod world;
pub mod resource;

pub struct Application {
    event_loop: EventLoop<()>,
    render_context: VulkanContext,
    layers: LayerVec
}

impl Application {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let layers = Arc::new(Mutex::new(vec![]));
        let render_context = VulkanContext::new_windowed(
            &event_loop,
            WindowBuilder::new()
                .with_title("proper")
                .with_resizable(false),
            layers.clone(),
        );

        let scene = Box::new(SceneLayer::new(
            render_context.gfx_queue().clone(),
            render_context.output_format(),
            render_context.swapchain_images(),
            render_context.viewport().clone(),
            render_context.dimensions(),
        ));
        layers.lock().unwrap().push(scene);

        let gui = Box::new(GuiLayer::new(
            render_context.surface().clone(),
            render_context.gfx_queue().clone(),
        ));
        layers.lock().unwrap().push(gui);

        Self {
            event_loop,
            render_context,
            layers
        }
    }

    pub fn run(mut self) {
        self.event_loop.run(move |event, _, flow| match event {
            winit::event::Event::WindowEvent { event, .. } => {
                if let WindowEvent::Resized(_) = event {
                    self.render_context.invalidate_surface();
                }

                // TODO there's no game logic, so quit event is handled right here
                if let WindowEvent::CloseRequested = event {
                    *flow = ControlFlow::Exit;
                    return;
                }

                if let Ok(event) = Event::try_from(&event) {
                    for layer in self.layers.lock().unwrap().iter_mut().rev() {
                        if layer.on_event(&event, flow) {
                            break;
                        }
                    }
                } else {
                    log::info!("Ignoring unhandled event: {:?}", event);
                }
            }
            winit::event::Event::RedrawEventsCleared => {
                self.render_context.do_frame(flow);
            }
            _ => (),
        });
    }
}

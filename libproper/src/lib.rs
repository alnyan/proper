#![allow(clippy::into_iter_on_ref)]

use std::sync::{Arc, Mutex};

use error::Error;
use event::{Event, GameEvent};
use render::{
    context::{LayerVec, VulkanContext},
    gui::GuiLayer,
    scene::SceneLayer,
};
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod error;
pub mod event;
pub mod layer;
pub mod render;
pub mod resource;
pub mod world;

pub struct Application {
    event_loop: EventLoop<GameEvent>,
    render_context: VulkanContext,
    layers: LayerVec,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        rayon::ThreadPoolBuilder::new().num_threads(24).build_global().unwrap();

        let event_loop = EventLoop::with_user_event();
        let proxy = event_loop.create_proxy();
        let layers = Arc::new(Mutex::new(vec![]));
        let render_context = VulkanContext::new_windowed(
            &event_loop,
            WindowBuilder::new()
                .with_title("proper")
                .with_resizable(false),
            layers.clone(),
        )?;

        let scene = Box::new(SceneLayer::new(
            render_context.gfx_queue().clone(),
            render_context.output_format(),
            render_context.swapchain_images(),
            render_context.viewport().clone(),
            render_context.dimensions(),
        )?);
        layers.lock().unwrap().push(scene);

        let gui = Box::new(GuiLayer::new(
            proxy.clone(),
            render_context.surface().clone(),
            render_context.gfx_queue().clone(),
        ));
        layers.lock().unwrap().push(gui);

        Ok(Self {
            event_loop,
            render_context,
            layers,
        })
    }

    pub fn run(mut self) {
        self.event_loop.run(move |event, _, flow| match event {
            winit::event::Event::UserEvent(event) => {
                Self::notify_layers(&self.layers, &Event::GameEvent(event), flow);
            }
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
                    Self::notify_layers(&self.layers, &event, flow);
                } else {
                    log::info!("Ignoring unhandled event: {:?}", event);
                }
            }
            winit::event::Event::RedrawEventsCleared => {
                self.render_context.do_frame(flow).unwrap();
            }
            _ => (),
        });
    }

    fn notify_layers(layers: &LayerVec, event: &Event, flow: &mut ControlFlow) {
        for layer in layers.lock().unwrap().iter_mut().rev() {
            if layer.on_event(&event, flow).unwrap() {
                break;
            }
        }
    }
}

#![allow(clippy::into_iter_on_ref)]

use std::sync::{Arc, Mutex};

use error::Error;
use event::{Event, GameEvent};
use layer::{gui::GuiLayer, logic::LogicLayer, world::WorldLayer, Layer};
use render::context::VulkanContext;
use resource::{material::MaterialRegistry, model::ModelRegistry, texture::TextureRegistry};
use vulkano::format::Format;
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use world::scene::Scene;

pub mod error;
pub mod event;
pub mod layer;
pub mod render;
pub mod resource;
pub mod world;

pub struct Application {
    event_loop: EventLoop<GameEvent>,
    render_context: VulkanContext,
    layers: Vec<Box<dyn Layer>>,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(24)
            .build_global()
            .unwrap();
        let event_loop = EventLoop::with_user_event();
        let proxy = event_loop.create_proxy();
        let mut layers: Vec<Box<dyn Layer>> = vec![];
        let render_context = VulkanContext::new_windowed(
            &event_loop,
            WindowBuilder::new()
                .with_title("proper")
                .with_resizable(false),
        )?;

        // TODO I still don't know where to place this lol
        let render_pass = vulkano::ordered_passes_renderpass!(
            render_context.gfx_queue().device().clone(),
            attachments: {
                ms_color: {
                    load: Clear,
                    store: DontCare,
                    format: render_context.output_format(),
                    samples: 4,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 4,
                },
                final_color: {
                    load: Clear,
                    store: Store,
                    format: render_context.output_format(),
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [ms_color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: [ms_color]
                }
            ]
        )
        .unwrap();

        let material_registry = Arc::new(Mutex::new(MaterialRegistry::new(
            render_context.gfx_queue().clone(),
            render_pass.clone(),
            render_context.viewport().clone(),
        )));
        let model_registry = Arc::new(Mutex::new(ModelRegistry::new(
            render_context.gfx_queue().clone(),
        )));
        let texture_registry = Arc::new(Mutex::new(TextureRegistry::new(
            render_context.gfx_queue().clone(),
        )?));
        let scene = Arc::new(Mutex::new(Scene::default()));

        let world_layer = Box::new(WorldLayer::new(
            render_context.gfx_queue().clone(),
            render_pass,
            material_registry.clone(),
            render_context.swapchain_images(),
            render_context.viewport().clone(),
            render_context.dimensions(),
            scene.clone(),
        )?);
        layers.push(world_layer);

        let gui = Box::new(GuiLayer::new(
            proxy,
            render_context.surface().clone(),
            render_context.gfx_queue().clone(),
        ));
        layers.push(gui);

        let logic_layer = Box::new(LogicLayer::new(
            scene,
            material_registry,
            model_registry,
            texture_registry,
        ));
        layers.push(logic_layer);

        Ok(Self {
            event_loop,
            render_context,
            layers,
        })
    }

    pub fn run(mut self) {
        self.event_loop.run(move |event, _, flow| match event {
            winit::event::Event::UserEvent(event) => {
                Self::notify_layers(&mut self.layers, &Event::GameEvent(event), flow);
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
                    Self::notify_layers(&mut self.layers, &event, flow);
                } else {
                    log::info!("Ignoring unhandled event: {:?}", event);
                }
            }
            winit::event::Event::RedrawEventsCleared => {
                self.render_context
                    .do_frame(flow, &mut self.layers)
                    .unwrap();
            }
            _ => (),
        });
    }

    fn notify_layers(layers: &mut Vec<Box<dyn Layer>>, event: &Event, flow: &mut ControlFlow) {
        for layer in layers.iter_mut().rev() {
            if layer.on_event(event, flow).unwrap() {
                break;
            }
        }
    }
}

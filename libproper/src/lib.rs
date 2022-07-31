#![allow(clippy::into_iter_on_ref)]

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use error::Error;
use event::{Event, GameEvent};
use layer::{gui::GuiLayer, logic::LogicLayer, world::WorldLayer, LayerManager, input::InputLayer};
use render::context::VulkanContext;
use resource::{material::MaterialRegistry, model::ModelRegistry, texture::TextureRegistry};
use vulkano::format::Format;
use winit::{
    event::{DeviceEvent, WindowEvent},
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
    layer_manager: LayerManager
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(24)
            .build_global()
            .unwrap();
        let event_loop = EventLoop::with_user_event();
        let proxy = event_loop.create_proxy();
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

        let gui = Box::new(GuiLayer::new(
            proxy.clone(),
            render_context.surface().clone(),
            render_context.gfx_queue().clone(),
            scene.clone(),
        ));

        let input_layer = Box::new(InputLayer::new(proxy.clone()));
        let logic_layer = Box::new(LogicLayer::new(
            proxy,
            scene,
            material_registry,
            model_registry,
            texture_registry,
            input_layer.state.clone(),
        ));

        let mut layer_manager = LayerManager::default();
        layer_manager.push(world_layer);
        layer_manager.push(logic_layer);
        layer_manager.push(input_layer);
        layer_manager.push(gui);

        Ok(Self {
            event_loop,
            render_context,
            layer_manager
        })
    }

    pub fn run(mut self) {
        let mut t0 = Instant::now();
        let mut mouse_grabbed = false;

        self.event_loop.run(move |event, _, flow| {
            let t = Instant::now();
            let delta = (t - t0).as_secs_f64();
            t0 = t;

            self.layer_manager.tick(delta).unwrap();

            match event {
                winit::event::Event::DeviceEvent { event, .. } => {
                    if mouse_grabbed {
                        if let DeviceEvent::MouseMotion { delta } = event {
                            self.layer_manager.notify_all(&Event::MouseMotion(delta), flow).unwrap();
                        }
                    }
                }
                winit::event::Event::UserEvent(event) => {
                    // TODO WindowLayer
                    if let GameEvent::SetMouseGrab(grab) = event {
                        if grab {
                            self.render_context.window().set_cursor_grab(true).unwrap();
                            self.render_context.window().set_cursor_visible(false);
                        } else {
                            self.render_context.window().set_cursor_grab(false).unwrap();
                            self.render_context.window().set_cursor_visible(true);
                        }
                        mouse_grabbed = grab;
                    }

                    self.layer_manager.notify_all(&Event::GameEvent(event), flow).unwrap();
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

                    if let WindowEvent::CursorMoved { .. } = event && mouse_grabbed {
                        return;
                    }

                    if let Ok(event) = Event::try_from(&event) {
                        self.layer_manager.notify_all(&event, flow).unwrap();
                    } else {
                        log::info!("Ignoring unhandled event: {:?}", event);
                    }
                }
                winit::event::Event::RedrawEventsCleared => {
                    self.render_context
                        .do_frame(flow, &mut self.layer_manager)
                        .unwrap();
                }
                _ => (),
            }
        });
    }
}

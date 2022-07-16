use render::context::VulkanContext;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod render;

pub struct Application {
    event_loop: EventLoop<()>,
    render_context: VulkanContext,
}

impl Application {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let render_context = VulkanContext::new_windowed(
            &event_loop,
            WindowBuilder::new()
                .with_title("proper")
                .with_resizable(false),
        );

        Self {
            event_loop,
            render_context,
        }
    }

    pub fn run(mut self) {
        self.event_loop.run(move |event, _, flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                self.render_context.invalidate_surface();
            }
            Event::RedrawEventsCleared => {
                self.render_context.do_frame();
            }
            _ => (),
        });
    }
}

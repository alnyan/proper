use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub struct Application {
    event_loop: EventLoop<()>
}

impl Application {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();

        Self {
            event_loop
        }
    }

    pub fn run(self) {
        self.event_loop.run(|event, _, flow| match event {
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
                todo!()
            }
            _ => (),
        });
    }
}

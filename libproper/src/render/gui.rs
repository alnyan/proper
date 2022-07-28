use std::sync::Arc;

use egui_winit_vulkano::{egui, Gui};
use vulkano::{device::Queue, swapchain::Surface, sync::GpuFuture};
use winit::{event_loop::{ControlFlow, EventLoopProxy}, window::Window};

use crate::{error::Error, event::{Event, GameEvent}, layer::Layer};

use super::frame::Frame;

pub struct GuiLayer {
    inner: Gui,
    event_proxy: EventLoopProxy<GameEvent>,
}

impl GuiLayer {
    pub fn new(event_proxy: EventLoopProxy<GameEvent>, surface: Arc<Surface<Window>>, gfx_queue: Arc<Queue>) -> Self {
        let inner = Gui::new(surface, gfx_queue, true);
        Self { inner, event_proxy }
    }
}

impl Layer for GuiLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_event(&mut self, event: &Event, _: &mut ControlFlow) -> Result<bool, Error> {
        if let Event::WindowEventWrapped(event) = event {
            Ok(self.inner.update(event))
        } else {
            Ok(false)
        }
    }

    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error> {
        self.inner.immediate_ui(|gui| {
            let ctx = gui.context();

            egui::SidePanel::new(egui::panel::Side::Left, 0)
                .max_width(128.0)
                .resizable(true)
                .show(&ctx, |ui| {
                    if ui.add(egui::Button::new("TEXT")).clicked() {
                        self.event_proxy.send_event(GameEvent::TestEvent).ok();
                    }
                });
        });

        Ok(self
            .inner
            .draw_on_image(in_future, frame.destination.clone()))
    }
}

use std::sync::Arc;

use egui_winit_vulkano::{egui, Gui};
use vulkano::{device::Queue, swapchain::Surface, sync::GpuFuture};
use winit::{event_loop::ControlFlow, window::Window};

use crate::{event::Event, layer::Layer};

use super::frame::Frame;

pub struct GuiLayer {
    inner: Gui,
}

impl GuiLayer {
    pub fn new(surface: Arc<Surface<Window>>, gfx_queue: Arc<Queue>) -> Self {
        let inner = Gui::new(surface, gfx_queue, false);
        Self { inner }
    }
}

impl Layer for GuiLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_event(&mut self, event: &Event, _: &mut ControlFlow) -> bool {
        if let Event::WindowEventWrapped(event) = event {
            self.inner.update(event)
        } else {
            false
        }
    }

    fn on_draw(&mut self, in_future: Box<dyn GpuFuture>, frame: &Frame) -> Box<dyn GpuFuture> {
        self.inner.immediate_ui(|gui| {
            let ctx = gui.context();

            egui::CentralPanel::default().show(&ctx, |ui| {
                if ui.add(egui::Button::new("This is a button")).clicked() {
                    println!("Button clicked!");
                }
            });
        });

        self.inner
            .draw_on_image(in_future, frame.destination.clone())
    }
}

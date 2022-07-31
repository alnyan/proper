use std::sync::{Arc, Mutex};

use egui_winit_vulkano::{egui, Gui};
use vulkano::{device::Queue, swapchain::Surface, sync::GpuFuture};
use winit::{
    event_loop::{ControlFlow, EventLoopProxy},
    window::Window,
};

use crate::{
    error::Error,
    event::{Event, GameEvent},
    layer::Layer,
    render::frame::Frame,
    world::scene::Scene,
};

pub struct GuiLayer {
    inner: Gui,
    scene: Arc<Mutex<Scene>>,
    event_proxy: EventLoopProxy<GameEvent>,
}

impl GuiLayer {
    pub fn new(
        event_proxy: EventLoopProxy<GameEvent>,
        surface: Arc<Surface<Window>>,
        gfx_queue: Arc<Queue>,
        scene: Arc<Mutex<Scene>>,
    ) -> Self {
        let inner = Gui::new(surface, None, gfx_queue, true);
        Self {
            inner,
            event_proxy,
            scene,
        }
    }
}

impl Layer for GuiLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn on_tick(&mut self, _delta: f64) -> Result<(), Error> {
        Ok(())
    }

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
                .min_width(200.0)
                .max_width(200.0)
                .resizable(true)
                .show(&ctx, |ui| {
                    if ui.add(egui::Button::new("TEXT")).clicked() {
                        self.event_proxy.send_event(GameEvent::TestEvent).ok();
                    }
                    let scene = self.scene.lock().unwrap();
                    let camera_position = scene.camera.position();
                    let camera_pitch = scene.camera.pitch();
                    let camera_yaw = scene.camera.yaw();
                    ui.add(egui::Label::new(format!(
                        "Position: {:.3}, {:.3}, {:.3}",
                        camera_position.x, camera_position.y, camera_position.z
                    )));

                    ui.add(egui::Label::new(format!(
                        "Pitch: {:.3}°, Yaw: {:.3}°",
                        camera_pitch.to_degrees(), camera_yaw.to_degrees()
                    )))
                });
        });

        Ok(self
            .inner
            .draw_on_image(in_future, frame.destination.clone()))
    }
}

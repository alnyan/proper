use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use vulkano::sync::GpuFuture;
use winit::{
    event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoopProxy},
};

use crate::{
    error::Error,
    event::{Event, GameEvent},
    render::frame::Frame,
};

use super::Layer;

#[derive(Default)]
pub struct InputState {
    pub forward: AtomicBool,
    pub back: AtomicBool,
    pub left: AtomicBool,
    pub right: AtomicBool,
    pub up: AtomicBool,
    pub down: AtomicBool,
}

pub struct InputLayer {
    event_proxy: EventLoopProxy<GameEvent>,
    pub state: Arc<InputState>,
    mouse_grab_state: bool,
}

impl InputLayer {
    pub fn new(event_proxy: EventLoopProxy<GameEvent>) -> Self {
        Self {
            event_proxy,
            mouse_grab_state: false,
            state: Default::default(),
        }
    }

    pub fn handle_key_input(&mut self, input: &KeyboardInput) -> Result<bool, Error> {
        let state = input.state == ElementState::Pressed;
        match input.virtual_keycode {
            Some(VirtualKeyCode::W) => self.state.forward.store(state, Ordering::Release),
            Some(VirtualKeyCode::S) => self.state.back.store(state, Ordering::Release),
            Some(VirtualKeyCode::A) => self.state.left.store(state, Ordering::Release),
            Some(VirtualKeyCode::D) => self.state.right.store(state, Ordering::Release),
            Some(VirtualKeyCode::Space) => self.state.up.store(state, Ordering::Release),
            Some(VirtualKeyCode::LControl) => self.state.down.store(state, Ordering::Release),
            Some(VirtualKeyCode::Escape) => {
                if self.mouse_grab_state {
                    self.mouse_grab_state = false;
                    self.event_proxy
                        .send_event(GameEvent::SetMouseGrab(false))
                        .unwrap();
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn handle_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> Result<bool, Error> {
        if state == ElementState::Pressed && button == MouseButton::Left {
            self.mouse_grab_state = true;
            self.event_proxy
                .send_event(GameEvent::SetMouseGrab(true))
                .unwrap();

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Layer for InputLayer {
    fn on_attach(&mut self) {}
    fn on_detach(&mut self) {}

    fn on_draw(
        &mut self,
        in_future: Box<dyn GpuFuture>,
        _frame: &Frame,
    ) -> Result<Box<dyn GpuFuture>, Error> {
        Ok(in_future)
    }

    fn on_tick(&mut self, _delta: f64) -> Result<(), Error> {
        Ok(())
    }

    fn on_event(&mut self, event: &Event, _flow: &mut ControlFlow) -> Result<bool, Error> {
        match event {
            Event::WindowEventWrapped(WindowEvent::KeyboardInput { input, .. }) => {
                self.handle_key_input(input)
            }
            Event::WindowEventWrapped(&WindowEvent::MouseInput { state, button, .. }) => {
                self.handle_mouse_input(button, state)
            }
            _ => Ok(false),
        }
    }
}

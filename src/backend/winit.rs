use crate::event::{Event, Key, Modifiers};

use winit::event::{DeviceEvent, ElementState, KeyboardInput, MouseButton, MouseScrollDelta, WindowEvent};

/// Converts a winit event to a pixel-widgets event, if such a conversion is available.
/// Requires the "winit" feature.
pub fn convert_event<T>(ev: winit::event::Event<T>) -> Option<Event> {
    match ev {
        winit::event::Event::WindowEvent { event, .. } => match event {
            WindowEvent::Resized(size) => Some(Event::Resize(size.width as f32, size.height as f32)),
            WindowEvent::CloseRequested => Some(Event::Exit),
            WindowEvent::Focused(f) => Some(Event::Focus(f)),
            WindowEvent::ReceivedCharacter(c) => Some(Event::Text(c)),
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                } => convert_key(key).map(Event::Press),
                KeyboardInput {
                    state: ElementState::Released,
                    virtual_keycode: Some(key),
                    ..
                } => convert_key(key).map(Event::Release),
                _ => None,
            },
            WindowEvent::ModifiersChanged(modifiers) => Some(Event::Modifiers(convert_mods(modifiers))),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => match button {
                MouseButton::Left => Some(Event::Press(Key::LeftMouseButton)),
                MouseButton::Right => Some(Event::Press(Key::RightMouseButton)),
                MouseButton::Middle => Some(Event::Press(Key::MiddleMouseButton)),
                MouseButton::Other(_) => None,
            },
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => match button {
                MouseButton::Left => Some(Event::Release(Key::LeftMouseButton)),
                MouseButton::Right => Some(Event::Release(Key::RightMouseButton)),
                MouseButton::Middle => Some(Event::Release(Key::MiddleMouseButton)),
                MouseButton::Other(_) => None,
            },
            WindowEvent::CursorMoved { position, .. } => Some(Event::Cursor(position.x as f32, position.y as f32)),
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(dx, dy) => Some(Event::Scroll(dx * 20.0, dy * 20.0)),

                MouseScrollDelta::PixelDelta(delta) => Some(Event::Scroll(delta.x as f32, delta.y as f32)),
            },
            _ => None,
        },
        winit::event::Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta: (x, y) },
            ..
        } => Some(Event::Motion(x as f32, y as f32)),
        _ => None,
    }
}

fn convert_mods(x: winit::event::ModifiersState) -> Modifiers {
    Modifiers {
        ctrl: x.ctrl(),
        alt: x.alt(),
        shift: x.shift(),
        logo: x.logo(),
        #[cfg(target_os = "macos")]
        command: x.logo(),
        #[cfg(not(target_os = "macos"))]
        command: x.ctrl(),
    }
}

fn convert_key(key: winit::event::VirtualKeyCode) -> Option<Key> {
    use winit::event::VirtualKeyCode as Vk;

    match key {
        Vk::Key1 => Some(Key::Key1),
        Vk::Key2 => Some(Key::Key2),
        Vk::Key3 => Some(Key::Key3),
        Vk::Key4 => Some(Key::Key4),
        Vk::Key5 => Some(Key::Key5),
        Vk::Key6 => Some(Key::Key6),
        Vk::Key7 => Some(Key::Key7),
        Vk::Key8 => Some(Key::Key8),
        Vk::Key9 => Some(Key::Key9),
        Vk::Key0 => Some(Key::Key0),
        Vk::F1 => Some(Key::F1),
        Vk::F2 => Some(Key::F2),
        Vk::F3 => Some(Key::F3),
        Vk::F4 => Some(Key::F4),
        Vk::F5 => Some(Key::F5),
        Vk::F6 => Some(Key::F6),
        Vk::F7 => Some(Key::F7),
        Vk::F8 => Some(Key::F8),
        Vk::F9 => Some(Key::F9),
        Vk::F10 => Some(Key::F10),
        Vk::F11 => Some(Key::F11),
        Vk::F12 => Some(Key::F12),
        Vk::A => Some(Key::A),
        Vk::B => Some(Key::B),
        Vk::C => Some(Key::C),
        Vk::D => Some(Key::D),
        Vk::E => Some(Key::E),
        Vk::F => Some(Key::F),
        Vk::G => Some(Key::G),
        Vk::H => Some(Key::H),
        Vk::I => Some(Key::I),
        Vk::J => Some(Key::J),
        Vk::K => Some(Key::K),
        Vk::L => Some(Key::L),
        Vk::M => Some(Key::M),
        Vk::N => Some(Key::N),
        Vk::O => Some(Key::O),
        Vk::P => Some(Key::P),
        Vk::Q => Some(Key::Q),
        Vk::R => Some(Key::R),
        Vk::S => Some(Key::S),
        Vk::T => Some(Key::T),
        Vk::U => Some(Key::U),
        Vk::V => Some(Key::V),
        Vk::W => Some(Key::W),
        Vk::X => Some(Key::X),
        Vk::Y => Some(Key::Y),
        Vk::Z => Some(Key::Z),
        Vk::Tab => Some(Key::Tab),
        Vk::LShift => Some(Key::Shift),
        Vk::LControl => Some(Key::Ctrl),
        Vk::LAlt => Some(Key::Alt),
        Vk::Space => Some(Key::Space),
        Vk::Return => Some(Key::Enter),
        Vk::Back => Some(Key::Backspace),
        Vk::Escape => Some(Key::Escape),
        Vk::Left => Some(Key::Left),
        Vk::Right => Some(Key::Right),
        Vk::Up => Some(Key::Up),
        Vk::Down => Some(Key::Down),
        Vk::Home => Some(Key::Home),
        Vk::End => Some(Key::End),
        Vk::Minus => Some(Key::Minus),
        Vk::Plus => Some(Key::Plus),
        Vk::LBracket => Some(Key::BracketOpen),
        Vk::RBracket => Some(Key::BracketClose),
        Vk::Comma => Some(Key::Comma),
        Vk::Period => Some(Key::Period),
        Vk::Semicolon => Some(Key::Semicolon),
        Vk::Apostrophe => Some(Key::Quote),
        Vk::Grave => Some(Key::Tilde),
        Vk::Backslash => Some(Key::Backslash),
        Vk::Slash => Some(Key::Slash),
        _ => None,
    }
}

/// A key
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    LeftMouseButton,
    MiddleMouseButton,
    RightMouseButton,

    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Tab,
    Shift,
    Ctrl,
    Alt,
    Space,
    Enter,
    Backspace,
    Escape,
    Home,
    End,

    Left,
    Right,
    Up,
    Down,
}

/// A set of modifiers
#[derive(Clone, Copy, Debug)]
pub struct Modifiers {
    /// `true` if the control key is pressed, `false otherwise.
    pub ctrl: bool,
    /// `true` if the alt key is pressed, `false otherwise.
    pub alt: bool,
    /// `true` if the shift key is pressed, `false otherwise.
    pub shift: bool,
    /// `true` if the windows/super/command key is pressed, `false otherwise.
    pub logo: bool,
}

#[allow(missing_docs)]
impl Modifiers {
    pub fn none() -> Modifiers {
        Modifiers {
            ctrl: false,
            alt: false,
            shift: false,
            logo: false,
        }
    }

    pub fn ctrl() -> Modifiers {
        Modifiers {
            ctrl: true,
            alt: false,
            shift: false,
            logo: false,
        }
    }

    pub fn alt() -> Modifiers {
        Modifiers {
            ctrl: false,
            alt: true,
            shift: false,
            logo: false,
        }
    }

    pub fn shift() -> Modifiers {
        Modifiers {
            ctrl: false,
            alt: false,
            shift: true,
            logo: false,
        }
    }

    pub fn logo() -> Modifiers {
        Modifiers {
            ctrl: false,
            alt: false,
            shift: false,
            logo: true,
        }
    }
}

/// A user input event.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// A button on some input device was pressed.
    Press(Key),
    /// A button on some input device was released.
    Release(Key),
    /// Modifiers were changed.
    Modifiers(Modifiers),
    /// The window was resized to the given dimensions.
    Resize(f32, f32),
    /// Some motion input was received (e.g. moving mouse or joystick axis).
    Motion(f32, f32),
    /// The mouse cursor was moved to a location.
    Cursor(f32, f32),
    /// The mouse wheel or touchpad scroll gesture sent us some scroll event.
    Scroll(f32, f32),
    /// Text input was received, usually via the keyboard.
    Text(char),
    /// The window was focused or lost focus.
    Focus(bool),
    /// The application exited it's main event loop
    Exit,
}

/// Events that can be subscribed to on a node
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeEvent {
    /// Occurs when the mouse starts hovering the widget
    MouseEnter,
    /// Occurs when the mouse stops hovering the widget
    MouseLeave,
    /// Occurs when the mouse moves while hovering the widget
    MouseMove,
    /// Occurs when a mouse button goes down while hovering the widget
    MouseDown(Key),
    /// Occurs when a mouse button goes up while hovering the widget
    MouseUp(Key),
    /// Occurs when a mouse button goes up while hovering the widget, but only if the mouse didn't stop hovering for
    /// the entire duration of the click.
    MouseClick(Key),
}

use std::mem::replace;

use smallvec::smallvec;

use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::{StyleState, Stylesheet};
use crate::widget::{Context, StateVec, Widget};

/// State for [`Toggle`](struct.Toggle.html)
#[allow(missing_docs)]
pub enum State {
    Idle,
    Hover,
    Pressed,
    Disabled,
}

/// A clickable button that toggles some `bool`.
pub struct Toggle<T, F: Fn(bool) -> T> {
    checked: bool,
    on_toggle: F,
}

impl<'a, T: 'a, F: 'a + Fn(bool) -> T> Toggle<T, F> {
    /// Constructs a new `Toggle`
    pub fn new<C: IntoNode<'a, T> + 'a>(checked: bool, on_toggle: F) -> Self {
        Self { checked, on_toggle }
    }

    /// Sets the current toggle state of the `Toggle`.
    pub fn val(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the on_toggle callback for this `Toggle`, which is called when the toggle state changes.
    pub fn on_toggle<N: Fn(bool) -> T>(self, on_toggle: N) -> Toggle<T, N> {
        Toggle {
            checked: self.checked,
            on_toggle,
        }
    }
}

impl<'a, T: 'a> Default for Toggle<T, fn(bool) -> T> {
    fn default() -> Self {
        Self {
            checked: false,
            on_toggle: |_| panic!("on_toggle of `Toggle` must be set"),
        }
    }
}

impl<'a, T, F: Send + Fn(bool) -> T> Widget<'a, T> for Toggle<T, F> {
    type State = State;

    fn mount(&self) -> Self::State {
        State::Idle
    }

    fn widget(&self) -> &'static str {
        "toggle"
    }

    fn state(&self, state: &State) -> StateVec {
        let mut state = match state {
            State::Idle => StateVec::new(),
            State::Hover => smallvec![StyleState::Hover],
            State::Pressed => smallvec![StyleState::Pressed],
            State::Disabled => smallvec![StyleState::Disabled],
        };

        if self.checked {
            state.push(StyleState::Checked);
        }

        state
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &State, stylesheet: &Stylesheet) -> (Size, Size) {
        match stylesheet.background {
            Background::Patch(ref patch, _) => {
                let size = patch.minimum_size();
                (Size::Exact(size.0), Size::Exact(size.1))
            }
            Background::Image(ref image, _) => (Size::Exact(image.size.width()), Size::Exact(image.size.height())),
            _ => (stylesheet.width, stylesheet.height),
        }
    }

    fn event(
        &mut self,
        state: &mut State,
        layout: Rectangle,
        clip: Rectangle,
        _: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        match event {
            Event::Cursor(x, y) => {
                *state = match replace(state, State::Idle) {
                    State::Idle => {
                        if layout.point_inside(x, y) && clip.point_inside(x, y) {
                            context.redraw();
                            State::Hover
                        } else {
                            State::Idle
                        }
                    }
                    State::Hover => {
                        if layout.point_inside(x, y) && clip.point_inside(x, y) {
                            State::Hover
                        } else {
                            context.redraw();
                            State::Idle
                        }
                    }
                    State::Pressed => {
                        if layout.point_inside(x, y) && clip.point_inside(x, y) {
                            State::Pressed
                        } else {
                            context.redraw();
                            State::Idle
                        }
                    }
                    State::Disabled => State::Disabled,
                };
            }

            Event::Press(Key::LeftMouseButton) => {
                *state = match replace(state, State::Idle) {
                    State::Hover => {
                        context.redraw();
                        State::Pressed
                    }
                    other => other,
                };
            }

            Event::Release(Key::LeftMouseButton) => {
                *state = match replace(state, State::Idle) {
                    State::Pressed => {
                        context.redraw();
                        context.push((self.on_toggle)(!self.checked));
                        State::Hover
                    }
                    other => other,
                };
            }

            _ => (),
        }
    }

    fn draw(&mut self, _: &mut State, layout: Rectangle, _: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        stylesheet.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a + Send, F: 'a + Send + Fn(bool) -> T> IntoNode<'a, T> for Toggle<T, F> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

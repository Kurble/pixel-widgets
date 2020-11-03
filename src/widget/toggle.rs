use std::mem::replace;

use smallvec::smallvec;

use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::{StyleState, Stylesheet};
use crate::widget::{Context, IntoNode, Node, StateVec, Widget};

/// State for [`Toggle`](struct.Toggle.html)
#[allow(missing_docs)]
pub enum State {
    Idle,
    Hover,
    Pressed,
    Disabled,
}

/// A clickable button that toggles some `bool`.
pub struct Toggle<'a, T, F: Fn(bool) -> T> {
    checked: bool,
    state: &'a mut State,
    on_toggle: F,
}

impl<'a, T: 'a, F: 'a + Fn(bool) -> T> Toggle<'a, T, F> {
    /// Constructs a new `Toggle`
    pub fn new<C: IntoNode<'a, T> + 'a>(checked: bool, state: &'a mut State, on_toggle: F) -> Self {
        Self {
            checked,
            state,
            on_toggle,
        }
    }
}

impl<'a, T, F: Send + Fn(bool) -> T> Widget<'a, T> for Toggle<'a, T, F> {
    fn widget(&self) -> &'static str {
        "toggle"
    }

    fn state(&self) -> StateVec {
        let mut state = match self.state {
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

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, T>)) {}

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        match stylesheet.background {
            Background::Patch(ref patch, _) => {
                let size = patch.minimum_size();
                (Size::Exact(size.0), Size::Exact(size.1))
            }
            Background::Image(ref image, _) => (Size::Exact(image.size.width()), Size::Exact(image.size.height())),
            _ => (stylesheet.width, stylesheet.height),
        }
    }

    fn event(&mut self, layout: Rectangle, clip: Rectangle, _: &Stylesheet, event: Event, context: &mut Context<T>) {
        match event {
            Event::Cursor(x, y) => {
                *self.state = match replace(self.state, State::Idle) {
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
                *self.state = match replace(self.state, State::Idle) {
                    State::Hover => {
                        context.redraw();
                        State::Pressed
                    }
                    other => other,
                };
            }

            Event::Release(Key::LeftMouseButton) => {
                *self.state = match replace(self.state, State::Idle) {
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

    fn draw(&mut self, layout: Rectangle, _: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        stylesheet.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a + Send, F: 'a + Send + Fn(bool) -> T> IntoNode<'a, T> for Toggle<'a, T, F> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

use crate::draw::*;
use crate::element::{Context, Element, IntoNode, Node, Stylable};
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use std::mem::replace;

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

impl<'a, T, F: Fn(bool) -> T> Element<'a, T> for Toggle<'a, T, F> {
    fn element(&self) -> &'static str {
        "toggle"
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn Stylable<'a>)) {}

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
        let background = match self.checked {
            false => &stylesheet.background,
            true => &stylesheet.checked,
        };

        background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a, F: 'a + Fn(bool) -> T> IntoNode<'a, T> for Toggle<'a, T, F> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

use crate::draw::*;
use crate::element::{Element, IntoNode};
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use std::mem::replace;

pub type State = crate::element::button::State;

pub struct Toggle<'a, T, F: Fn(bool) -> T> {
    checked: bool,
    state: &'a mut State,
    on_toggle: F,
}

impl<'a, T: 'a, F: 'a + Fn(bool) -> T> Toggle<'a, T, F> {
    pub fn new<C: IntoNode<'a, T> + 'a>(checked: bool, state: &'a mut State, on_toggle: F) -> Self {
        Self {
            checked,
            state,
            on_toggle
        }
    }
}

impl<'a, T, F: Fn(bool) -> T> Element<'a, T> for Toggle<'a, T, F> {
    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        match stylesheet.background {
            Background::Patch(ref patch, _) => {
                let size = patch.minimum_size();
                (Size::Exact(size.0), Size::Exact(size.1))
            }
            Background::Image(ref image, _) => {
                (Size::Exact(image.size.width()), Size::Exact(image.size.height()))
            }
            _ => (stylesheet.width, stylesheet.height)
        }
    }

    fn event(&mut self, layout: Rectangle, _: &Stylesheet, event: Event) -> Option<T> {
        let mut result = None;
        match event {
            Event::Cursor(x, y) => {
                *self.state = match replace(self.state, State::Idle) {
                    State::Idle | State::Hover => if layout.point_inside(x, y) {
                        State::Hover
                    } else {
                        State::Idle
                    },
                    State::Pressed => if layout.point_inside(x, y) {
                        State::Pressed
                    } else {
                        State::Idle
                    },
                    State::Disabled => State::Disabled,
                };
            },

            Event::Press(Key::LeftMouseButton) => {
                *self.state = match replace(self.state, State::Idle) {
                    State::Hover => State::Pressed,
                    other => other,
                };
            }

            Event::Release(Key::LeftMouseButton) => {
                *self.state = match replace(self.state, State::Idle) {
                    State::Pressed => {
                        result.replace((self.on_toggle)(!self.checked));
                        State::Hover
                    },
                    other => other,
                };
            }

            _ => ()
        }

        result
    }

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let background = match self.checked {
            false => &stylesheet.background,
            true => &stylesheet.checked,
        };

        background
            .render(layout)
            .into_iter()
            .collect()
    }
}

impl<'a, T: 'a, F: 'a + Fn(bool) -> T> IntoNode<'a, T> for Toggle<'a, T, F> {}

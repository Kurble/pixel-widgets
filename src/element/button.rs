use crate::draw::*;
use crate::element::{Element, IntoNode, Node};
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use std::mem::replace;

pub struct Button<'a, T> {
    state: &'a mut State,
    content: Node<'a, T>,
    on_clicked: Option<T>,
}

pub enum State {
    Idle,
    Hover,
    Pressed,
    Disabled,
}

impl<'a, T: 'a> Button<'a, T> {
    pub fn new<C: IntoNode<'a, T> + 'a>(state: &'a mut State, content: C) -> Self {
        Self {
            state,
            content: content.into_node(),
            on_clicked: None,
        }
    }

    pub fn on_clicked(mut self, message: T) -> Self {
        self.on_clicked = Some(message);
        self
    }
}

impl<'a, T> Element<'a, T> for Button<'a, T> {
    fn element(&self) -> &'static str {
        "button"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        visitor(&mut self.content);
    }

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        let (content_width, content_height) = self.content.size();

        let padding = stylesheet.background.padding();
        let padding = Rectangle {
            left: padding.left + stylesheet.padding.left,
            right: padding.right + stylesheet.padding.right,
            top: padding.top + stylesheet.padding.top,
            bottom: padding.bottom + stylesheet.padding.bottom,
        };

        let resolve = |size, content_size, padding| match size {
            Size::Shrink => match content_size {
                Size::Fill(_) => Size::Shrink,
                Size::Exact(units) => Size::Exact(units + padding),
                Size::Shrink => Size::Shrink,
            },
            other => other,
        };

        (
            resolve(stylesheet.width, content_width, padding.left + padding.right),
            resolve(stylesheet.height, content_height, padding.top + padding.bottom),
        )
    }

    fn event(&mut self, layout: Rectangle, _: &Stylesheet, event: Event, clip: Rectangle) -> Option<T> {
        let mut result = None;
        match event {
            Event::Cursor(x, y) => {
                *self.state = match replace(self.state, State::Idle) {
                    State::Idle | State::Hover => {
                        if layout.point_inside(x, y) && clip.point_inside(x, y) {
                            State::Hover
                        } else {
                            State::Idle
                        }
                    }
                    State::Pressed => {
                        if layout.point_inside(x, y) && clip.point_inside(x, y) {
                            State::Pressed
                        } else {
                            State::Idle
                        }
                    }
                    State::Disabled => State::Disabled,
                };
            }

            Event::Press(Key::LeftMouseButton) => {
                *self.state = match replace(self.state, State::Idle) {
                    State::Hover => State::Pressed,
                    other => other,
                };
            }

            Event::Release(Key::LeftMouseButton) => {
                *self.state = match replace(self.state, State::Idle) {
                    State::Pressed => {
                        result = self.on_clicked.take();
                        State::Hover
                    }
                    other => other,
                };
            }

            _ => (),
        }

        result
    }

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let content_rect = stylesheet
            .background
            .content_rect(layout)
            .after_padding(stylesheet.padding);

        let background = match self.state {
            State::Idle => &stylesheet.background,
            State::Hover => &stylesheet.hover,
            State::Pressed => &stylesheet.pressed,
            State::Disabled => &stylesheet.disabled,
        };

        background
            .render(layout)
            .into_iter()
            .chain(self.content.render(content_rect).into_iter())
            .collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Button<'a, T> {}

impl Default for State {
    fn default() -> Self {
        State::Idle
    }
}

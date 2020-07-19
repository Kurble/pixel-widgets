use std::mem::replace;

use crate::draw::*;
use crate::widget::{Context, Widget, IntoNode, Node};
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

/// A clickable button
pub struct Button<'a, T> {
    state: &'a mut State,
    content: Node<'a, T>,
    on_clicked: Option<T>,
}

/// State for [`Button`](struct.Button.html)
#[allow(missing_docs)]
pub enum State {
    Idle,
    Hover,
    Pressed,
    Disabled,
}

impl<'a, T: 'a> Button<'a, T> {
    /// Construct a new button
    pub fn new<C: IntoNode<'a, T> + 'a>(state: &'a mut State, content: C) -> Self {
        Self {
            state,
            content: content.into_node(),
            on_clicked: None,
        }
    }

    /// Sets the message to be posted when this button is clicked.
    pub fn on_clicked(mut self, message: T) -> Self {
        self.on_clicked = Some(message);
        self
    }
}

impl<'a, T: 'a> Widget<'a, T> for Button<'a, T> {
    fn widget(&self) -> &'static str {
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
                        context.extend(self.on_clicked.take());
                        State::Hover
                    }
                    other => other,
                };
            }

            _ => (),
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
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
            .chain(self.content.draw(content_rect, clip).into_iter())
            .collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Button<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

impl Default for State {
    fn default() -> Self {
        State::Idle
    }
}

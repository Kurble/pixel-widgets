use std::mem::replace;

use smallvec::smallvec;

use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::{StyleState, Stylesheet};
use crate::widget::{Context, StateVec, Widget};

/// A clickable button
pub struct Button<'a, T> {
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
    pub fn new<C: IntoNode<'a, T> + 'a>(content: C) -> Self {
        Self {
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

impl<'a, T: 'a + Send> Widget<'a, T> for Button<'a, T> {
    type State = State;

    fn mount(&self) -> State {
        State::Idle
    }

    fn widget(&self) -> &'static str {
        "button"
    }

    fn state(&self, state: &State) -> StateVec {
        match state {
            State::Idle => StateVec::new(),
            State::Hover => smallvec![StyleState::Hover],
            State::Pressed => smallvec![StyleState::Pressed],
            State::Disabled => smallvec![StyleState::Disabled],
        }
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        visitor(&mut *self.content);
    }

    fn size(&self, _: &State, style: &Stylesheet) -> (Size, Size) {
        style
            .background
            .resolve_size((style.width, style.height), self.content.size(), style.padding)
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
                        context.extend(self.on_clicked.take());
                        State::Hover
                    }
                    other => other,
                };
            }

            _ => (),
        }
    }

    fn draw(&mut self, _: &mut State, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let content_rect = style.background.content_rect(layout, style.padding);

        style
            .background
            .render(layout)
            .into_iter()
            .chain(self.content.draw(content_rect, clip).into_iter())
            .collect()
    }
}

impl<'a, T: 'a + Send> IntoNode<'a, T> for Button<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

impl Default for State {
    fn default() -> Self {
        State::Idle
    }
}

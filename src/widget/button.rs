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
    content: Option<Node<'a, T>>,
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

impl<'a, T: 'a> Default for Button<'a, T> {
    fn default() -> Self {
        Self {
            content: None,
            on_clicked: None,
        }
    }
}

impl<'a, T: 'a> Button<'a, T> {
    /// Construct a new button
    pub fn new<C: IntoNode<'a, T> + 'a>(content: C) -> Self {
        Self {
            content: Some(content.into_node()),
            on_clicked: None,
        }
    }

    /// Sets the message to be posted when this button is clicked.
    pub fn on_clicked(mut self, message: T) -> Self {
        self.on_clicked = Some(message);
        self
    }

    /// Sets the content of the button to be a paragraph of text.
    pub fn text(mut self, text: impl Into<String> + 'a) -> Self {
        self.content = Some(text.into_node());
        self
    }

    /// Sets the content of the button from an iterator.
    /// Note that only the first element will be taken.
    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, T>>(mut self, iter: I) -> Self {
        if self.content.is_none() {
            self.content = iter.into_iter().next().map(IntoNode::into_node);
        }
        self
    }

    fn content(&self) -> &Node<'a, T> {
        self.content.as_ref().expect("content of `Button` must be set")
    }

    fn content_mut(&mut self) -> &mut Node<'a, T> {
        self.content.as_mut().expect("content of `Button` must be set")
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
        visitor(&mut **self.content_mut());
    }

    fn size(&self, _: &State, style: &Stylesheet) -> (Size, Size) {
        style
            .background
            .resolve_size((style.width, style.height), self.content().size(), style.padding)
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
            .chain(self.content_mut().draw(content_rect, clip).into_iter())
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

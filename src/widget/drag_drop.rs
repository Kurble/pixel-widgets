use std::cell::Cell;

use smallvec::smallvec;

use crate::draw::Primitive;
use crate::event::{Event, Key, NodeEvent};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::{Stylesheet, StyleState};
use crate::widget::{Context, Frame, IntoNode, Node, StateVec, Widget};

/// Message type for communicating between `Drag` and `Drop` widgets
pub trait DragDropId: 'static + Copy { }

/// Context for `Drag` and `Drop` widgets. Only `Drag` and `Drop` widgets that share the same `DragDropContext` can
/// interact with each other.
#[derive(Default, Clone)]
pub struct DragDropContext<T: DragDropId> {
    data: Cell<Option<T>>,
}

/// A draggable item that can be dropped in `Drop` zones.
pub struct Drag<'a, T: DragDropId, Message> {
    state: &'a mut DragState<T>,
    context: &'a DragDropContext<T>,
    data: T,
    content: Frame<'a, Message>,
}

/// State for `Drag`
#[derive(Default)]
pub struct DragState<T> {
    dragging: Option<T>,
    origin: (f32, f32),
    offset: (f32, f32),
}

/// A drop zone where draggable `Drag` items may be dropped
pub struct Drop<'a, T: DragDropId, Message, OnAccept, OnDrop> {
    state: &'a mut DropState<T>,
    context: &'a DragDropContext<T>,
    accept: OnAccept,
    drop: OnDrop,
    content: Frame<'a, Message>,
}

/// State for `Drop`
#[derive(Default)]
pub struct DropState<T> {
    hovering: Option<T>,
}

impl<'a, T: DragDropId, Message: 'a> Drag<'a, T, Message> {
    /// Construct a new `Drag` widget, with some data that is to be dragged through the context.
    pub fn new(
        state: &'a mut DragState<T>,
        context: &'a DragDropContext<T>,
        data: T,
        content: impl IntoNode<'a, Message>,
    ) -> Self {
        Self {
            state,
            context,
            data,
            content: Frame::new(content),
        }
    }
}

impl<'a, T: DragDropId, Message: 'a, OnAccept: 'a, OnDrop: 'a> Drop<'a, T, Message, OnAccept, OnDrop>
where
    OnAccept: Fn(T) -> bool,
    OnDrop: Fn(T, (f32, f32)) -> Message,
{
    /// Construct a new `Drop` widget
    pub fn new(
        state: &'a mut DropState<T>,
        context: &'a DragDropContext<T>,
        accept: OnAccept,
        drop: OnDrop,
        content: impl IntoNode<'a, Message>,
    ) -> Self {
        Self {
            state,
            context,
            accept,
            drop,
            content: Frame::new(content),
        }
    }
}

impl<'a, T: DragDropId, Message: 'a> Widget<'a, Message> for Drag<'a, T, Message> {
    fn widget(&self) -> &'static str {
        "drag"
    }

    fn state(&self) -> StateVec {
        if self.state.dragging.is_some() {
            smallvec![StyleState::Drag]
        } else {
            smallvec![]
        }
    }

    fn len(&self) -> usize {
        self.content.len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, Message>)) {
        self.content.visit_children(visitor);
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        self.content.size(style)
    }

    fn event(
        &mut self,
        _layout: Rectangle,
        _clip: Rectangle,
        _style: &Stylesheet,
        event: Event,
        context: &mut Context<Message>,
    ) {
        match event {
            Event::Cursor(x, y) if self.state.dragging.is_some() => {
                self.state.offset = (x - self.state.origin.0, y - self.state.origin.1);
                context.redraw();
            }

            Event::Release(Key::LeftMouseButton) if self.state.dragging.is_some() => {
                self.state.dragging.take();
                self.context.data.replace(None);
                context.redraw();
            }

            _ => (),
        }
    }

    fn node_event(&mut self, event: NodeEvent, context: &mut Context<Message>) {
        match event {
            NodeEvent::MouseDown(Key::LeftMouseButton) => {
                self.context.data.replace(Some(self.data));
                self.state.origin = context.cursor;
                self.state.offset = (0.0, 0.0);
                self.state.dragging = Some(self.data);
                context.redraw();
            }

            _ => (),
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        if self.state.dragging.is_some() {
            let (dx, dy) = self.state.offset;
            let mut result = Vec::new();
            result.push(Primitive::LayerUp);
            result.extend(self.content.draw(layout.translate(dx, dy), clip, style));
            result.push(Primitive::LayerDown);
            result
        } else {
            self.content.draw(layout, clip, style)
        }
    }
}

impl<'a, T: DragDropId, Message: 'a, OnAccept, OnDrop> Widget<'a, Message> for Drop<'a, T, Message, OnAccept, OnDrop>
where
    OnAccept: Fn(T) -> bool,
    OnDrop: Fn(T, (f32, f32)) -> Message,
{
    fn widget(&self) -> &'static str {
        "drop"
    }

    fn state(&self) -> StateVec {
        if self.state.hovering.is_some() {
            smallvec![StyleState::Drop]
        } else {
            smallvec![]
        }
    }

    fn len(&self) -> usize {
        self.content.len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, Message>)) {
        self.content.visit_children(visitor);
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        self.content.size(style)
    }

    fn node_event(&mut self, event: NodeEvent, context: &mut Context<Message>) {
        match event {
            NodeEvent::MouseEnter => {
                if let Some(data) = self.context.data.get() {
                    if (self.accept)(data) {
                        self.state.hovering = Some(data);
                    }
                }
            }
            NodeEvent::MouseLeave => {
                self.state.hovering = None;
            }
            NodeEvent::MouseUp(Key::LeftMouseButton) => {
                if let Some(data) = self.state.hovering.take() {
                    context.push((self.drop)(data, context.cursor));
                }
            }

            _ => (),
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        self.content.draw(layout, clip, style)
    }
}

impl<'a, T: DragDropId, Message: 'a> IntoNode<'a, Message> for Drag<'a, T, Message> {
    fn into_node(self) -> Node<'a, Message> {
        Node::new(self)
    }
}

impl<'a, T: DragDropId, Message: 'a, OnAccept: 'a, OnDrop: 'a> IntoNode<'a, Message> for Drop<'a, T, Message, OnAccept, OnDrop>
where
    OnAccept: Fn(T) -> bool,
    OnDrop: Fn(T, (f32, f32)) -> Message,
{
    fn into_node(self) -> Node<'a, Message> {
        Node::new(self)
    }
}

impl<T: 'static + Copy> DragDropId for T { }
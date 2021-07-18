#![allow(clippy::vec_init_then_push)]

use std::any::Any;
use std::sync::Mutex;

use smallvec::smallvec;

use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::{StyleState, Stylesheet};
use crate::widget::{Context, Frame, StateVec, Widget};

/// Message type for communicating between `Drag` and `Drop` widgets
pub trait DragDropId: 'static + Copy + Any + Send + Sync {}

/// Context for `Drag` and `Drop` widgets. Only `Drag` and `Drop` widgets that share the same `DragDropContext` can
/// interact with each other.
//#[derive(Clone)]
pub struct DragDropContext<T: DragDropId> {
    data: Mutex<Option<(T, (f32, f32))>>,
}

/// A draggable item that can be dropped in `Drop` zones.
pub struct Drag<'a, T: DragDropId, Message> {
    context: Option<&'a DragDropContext<T>>,
    data: Option<T>,
    content: Option<Frame<'a, Message>>,
}

/// State for `Drag`
pub struct DragState<T> {
    dragging: Option<T>,
    origin: (f32, f32),
    offset: (f32, f32),
}

/// A drop zone where draggable `Drag` items may be dropped
pub struct Drop<'a, T: DragDropId, Message, OnAccept, OnDrop> {
    context: Option<&'a DragDropContext<T>>,
    accept: OnAccept,
    drop: OnDrop,
    content: Option<Frame<'a, Message>>,
}

/// State for `Drop`
pub struct DropState<T> {
    hovering: Option<(T, (f32, f32))>,
    mouse_over: bool,
}

impl<'a, T: DragDropId, Message: 'a> Drag<'a, T, Message> {
    /// Construct a new `Drag` widget, with some data that is to be dragged through the context.
    pub fn new(context: &'a DragDropContext<T>, data: T, content: impl IntoNode<'a, Message>) -> Self {
        Self {
            context: Some(context),
            data: Some(data),
            content: Some(Frame::new(content)),
        }
    }

    pub fn context(mut self, context: &'a DragDropContext<T>) -> Self {
        self.context = Some(context);
        self
    }

    pub fn val(mut self, value: T) -> Self {
        self.data = Some(value);
        self
    }

    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, Message>>(mut self, iter: I) -> Self {
        if self.content.is_none() {
            self.content = iter.into_iter().next().map(Frame::new);
        }
        self
    }

    fn content(&self) -> &Frame<'a, Message> {
        self.content.as_ref().expect("content of `Drag` must be set")
    }

    fn content_mut(&mut self) -> &mut Frame<'a, Message> {
        self.content.as_mut().expect("content of `Drag` must be set")
    }
}

impl<'a, T: DragDropId, Message: 'a, OnAccept: 'a, OnDrop: 'a> Drop<'a, T, Message, OnAccept, OnDrop>
where
    OnAccept: Fn(T) -> bool,
    OnDrop: Fn(T, (f32, f32)) -> Message,
{
    /// Construct a new `Drop` widget
    pub fn new(
        context: &'a DragDropContext<T>,
        accept: OnAccept,
        drop: OnDrop,
        content: impl IntoNode<'a, Message>,
    ) -> Self {
        Self {
            context: Some(context),
            accept,
            drop,
            content: Some(Frame::new(content)),
        }
    }

    pub fn context(mut self, context: &'a DragDropContext<T>) -> Self {
        self.context = Some(context);
        self
    }

    pub fn on_accept<N: Fn(T) -> bool>(self, on_accept: N) -> Drop<'a, T, Message, N, OnDrop> {
        Drop {
            context: self.context,
            accept: on_accept,
            drop: self.drop,
            content: self.content,
        }
    }

    pub fn on_drop<N: Fn(T, (f32, f32)) -> Message>(self, on_drop: N) -> Drop<'a, T, Message, OnAccept, N> {
        Drop {
            context: self.context,
            accept: self.accept,
            drop: on_drop,
            content: self.content,
        }
    }

    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, Message>>(mut self, iter: I) -> Self {
        if self.content.is_none() {
            self.content = iter.into_iter().next().map(Frame::new);
        }
        self
    }

    fn content(&self) -> &Frame<'a, Message> {
        self.content.as_ref().expect("content of `Drop` must be set")
    }

    fn content_mut(&mut self) -> &mut Frame<'a, Message> {
        self.content.as_mut().expect("content of `Drop` must be set")
    }
}

impl<'a, T: DragDropId, Message> Default for Drag<'a, T, Message> {
    fn default() -> Self {
        Self {
            context: None,
            data: None,
            content: None,
        }
    }
}

impl<'a, T: DragDropId + Send + Sync, Message: 'a> Widget<'a, Message> for Drag<'a, T, Message> {
    type State = DragState<T>;

    fn mount(&self) -> Self::State {
        DragState::<T>::default()
    }

    fn widget(&self) -> &'static str {
        "drag"
    }

    fn state(&self, state: &DragState<T>) -> StateVec {
        if state.dragging.is_some() {
            smallvec![StyleState::Drag]
        } else {
            smallvec![]
        }
    }

    fn len(&self) -> usize {
        self.content().len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, Message>)) {
        self.content_mut().visit_children(visitor);
    }

    fn size(&self, _: &DragState<T>, style: &Stylesheet) -> (Size, Size) {
        self.content().size(&(), style)
    }

    fn event(
        &mut self,
        state: &mut DragState<T>,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<Message>,
    ) {
        match event {
            Event::Press(Key::LeftMouseButton) => {
                let (x, y) = context.cursor();
                if layout.point_inside(x, y) && clip.point_inside(x, y) {
                    self.context
                        .as_ref()
                        .expect("context of `Drag` must be set")
                        .data
                        .lock()
                        .unwrap()
                        .replace((
                            self.data.expect("data of `Drag` must be set"),
                            (context.cursor.0 - layout.left, context.cursor.1 - layout.top),
                        ));
                    state.origin = context.cursor;
                    state.offset = (0.0, 0.0);
                    state.dragging = Some(self.data.expect("data of `Drag` must be set"));
                    context.redraw();
                }
            }

            Event::Cursor(x, y) if state.dragging.is_some() => {
                state.offset = (x - state.origin.0, y - state.origin.1);
                context.redraw();
            }

            Event::Release(Key::LeftMouseButton) if state.dragging.is_some() => {
                state.dragging.take();
                self.context
                    .as_ref()
                    .expect("context of `Drag` must be set")
                    .data
                    .lock()
                    .unwrap()
                    .take();
                context.redraw();
            }

            _ => (),
        }

        self.content_mut().event(&mut (), layout, clip, style, event, context);
    }

    fn draw(
        &mut self,
        state: &mut DragState<T>,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
    ) -> Vec<Primitive<'a>> {
        if state.dragging.is_some() {
            let (dx, dy) = state.offset;
            let mut result = Vec::new();
            result.push(Primitive::LayerUp);
            result.extend(self.content_mut().draw(&mut (), layout.translate(dx, dy), clip, style));
            result.push(Primitive::LayerDown);
            result
        } else {
            self.content_mut().draw(&mut (), layout, clip, style)
        }
    }
}

impl<'a, T: DragDropId, Message> Default for Drop<'a, T, Message, fn(T) -> bool, fn(T, (f32, f32)) -> Message> {
    fn default() -> Self {
        Self {
            context: None,
            accept: |_| true,
            drop: |_, _| panic!("on_drop of `Drop` must be set"),
            content: None,
        }
    }
}

impl<'a, T, Message: 'a, OnAccept, OnDrop> Widget<'a, Message> for Drop<'a, T, Message, OnAccept, OnDrop>
where
    T: DragDropId + Send + Sync,
    OnAccept: 'a + Send + Fn(T) -> bool,
    OnDrop: 'a + Send + Fn(T, (f32, f32)) -> Message,
{
    type State = DropState<T>;

    fn mount(&self) -> DropState<T> {
        DropState::<T>::default()
    }

    fn widget(&self) -> &'static str {
        "drop"
    }

    fn state(&self, state: &DropState<T>) -> StateVec {
        if state.hovering.is_some() {
            smallvec![StyleState::Drop]
        } else if state.mouse_over {
            smallvec![StyleState::DropDenied]
        } else {
            smallvec![]
        }
    }

    fn len(&self) -> usize {
        self.content().len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, Message>)) {
        self.content_mut().visit_children(visitor);
    }

    fn size(&self, _: &DropState<T>, style: &Stylesheet) -> (Size, Size) {
        self.content().size(&(), style)
    }

    fn event(
        &mut self,
        state: &mut DropState<T>,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<Message>,
    ) {
        match event {
            Event::Cursor(x, y) => {
                let inside = layout.point_inside(x, y) && clip.point_inside(x, y);
                if inside && !state.mouse_over {
                    if let Some(data) = *self
                        .context
                        .as_ref()
                        .expect("context of `Drop` must be set")
                        .data
                        .lock()
                        .unwrap()
                    {
                        if (self.accept)(data.0) {
                            state.hovering = Some(data);
                        }
                    }
                } else if !inside && state.mouse_over {
                    state.hovering = None;
                }
                state.mouse_over = inside;
            }

            Event::Release(Key::LeftMouseButton) => {
                if let Some(data) = state.hovering.take() {
                    context.push((self.drop)(
                        data.0,
                        (
                            context.cursor.0 - (data.1).0 - layout.left,
                            context.cursor.1 - (data.1).1 - layout.top,
                        ),
                    ));
                }
            }

            _ => (),
        }

        self.content_mut().event(&mut (), layout, clip, style, event, context)
    }

    fn draw(
        &mut self,
        _: &mut DropState<T>,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
    ) -> Vec<Primitive<'a>> {
        self.content_mut().draw(&mut (), layout, clip, style)
    }
}

impl<'a, T: DragDropId + Send + Sync, Message: 'a> IntoNode<'a, Message> for Drag<'a, T, Message> {
    fn into_node(self) -> Node<'a, Message> {
        Node::from_widget(self)
    }
}

impl<'a, T: DragDropId + Send + Sync, Message: 'a, OnAccept: 'a, OnDrop: 'a> IntoNode<'a, Message>
    for Drop<'a, T, Message, OnAccept, OnDrop>
where
    OnAccept: Send + Fn(T) -> bool,
    OnDrop: Send + Fn(T, (f32, f32)) -> Message,
{
    fn into_node(self) -> Node<'a, Message> {
        Node::from_widget(self)
    }
}

impl<T: 'static + Copy + Send + Sync> DragDropId for T {}

impl<T: DragDropId> Default for DragDropContext<T> {
    fn default() -> Self {
        Self { data: Mutex::new(None) }
    }
}

impl<T: DragDropId> Default for DragState<T> {
    fn default() -> Self {
        Self {
            dragging: None,
            origin: (0.0, 0.0),
            offset: (0.0, 0.0),
        }
    }
}

impl<T: DragDropId> Default for DropState<T> {
    fn default() -> Self {
        Self {
            hovering: None,
            mouse_over: false,
        }
    }
}

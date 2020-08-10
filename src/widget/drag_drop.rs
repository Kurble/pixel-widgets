use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::widget::Context;

/// Context for `Drag` and `Drop` widgets. Only `Drag` and `Drop` widgets that share the same `DragDropContext` can
/// interact with each other.
#[derive(Default, Clone)]
pub struct DragDropContext<T> {
    data: Arc<Mutex<Option<T>>>,
}

/// A draggable item that can be dropped in `Drop` zones.
pub struct Drag<'a, T> {
    state: &'a mut DragState,
    context: DragDropContext<T>,
    data: T,
}

/// State for `Drag`
#[derive(Default)]
pub struct DragState {
    dragging: bool,
}

/// A drop zone where draggable `Drag` items may be dropped
pub struct Drop<T, Message, OnAccept, OnDrop> {
    context: DragDropContext<T>,
    accept: OnAccept,
    drop: OnDrop,
    marker: PhantomData<Message>,
}

impl<'a, T> Drag<'a, T> {
    pub fn new(state: &'a mut DragState, context: DragDropContext<T>, data: T) -> Self {
        Self {
            state,
            context,
            data,
        }
    }
}

impl<T, Message, OnAccept, OnDrop> Drop<T, Message, OnAccept, OnDrop>
where
    OnAccept: Fn(&mut T, &mut Context<Message>) -> bool,
    OnDrop: Fn(T) -> Message,
{
    pub fn new(context: DragDropContext<T>, accept: OnAccept, drop: OnDrop) -> Self {
        Self {
            context,
            accept,
            drop,
            marker: PhantomData,
        }
    }
}
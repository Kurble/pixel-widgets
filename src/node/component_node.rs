use std::cell::{Cell, RefCell, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;
use std::task::Poll;

use futures::{FutureExt, Stream, StreamExt};

use crate::component::{Component, Context};
use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, Node};
use crate::stylesheet::tree::Query;
use crate::tracker::{ManagedState, ManagedStateTracker};
use crate::widget::Context as WidgetContext;

pub struct ComponentNode<'a, M: 'a + Component> {
    props: Box<M>,
    state: RefCell<Option<&'a mut ManagedState>>,
    view: RefCell<Option<Node<'a, M::Message>>>,
    component_state: Cell<*mut (M::State, Runtime<M::Message>)>,
    style_query: Option<Query>,
    style_position: Option<(usize, usize)>,
    key: u64,
}

pub struct Runtime<Message> {
    futures: Vec<Box<dyn Future<Output = Message> + Send + Sync + Unpin>>,
    streams: Vec<Box<dyn Stream<Item = Message> + Send + Sync + Unpin>>,
    modified: bool,
}

/// Mutable state accessor.
/// By wrapping the mutable reference, the runtime knows if the state was mutated at all.
pub struct State<'a, T> {
    inner: &'a mut T,
    dirty: &'a mut bool,
}

impl<'a, M: 'a + Component> ComponentNode<'a, M> {
    pub fn new(props: M) -> Self {
        let mut hasher = DefaultHasher::new();
        std::any::type_name::<M>().hash(&mut hasher);
        Self {
            props: Box::new(props),
            state: RefCell::new(None),
            view: RefCell::new(None),
            component_state: Cell::new(null_mut()),
            style_query: None,
            style_position: None,
            key: hasher.finish(),
        }
    }

    pub fn dirty(&self) -> bool {
        self.view.borrow().is_none()
    }

    pub fn set_dirty(&self) {
        self.view.replace(None);
    }

    pub fn props(&self) -> &M {
        self.props.as_ref()
    }

    pub fn props_mut(&mut self) -> &mut M {
        self.set_dirty();
        self.props.as_mut()
    }

    pub fn update(&mut self, message: M::Message, context: &mut WidgetContext<M::Output>) {
        let mut dirty = false;

        let (state, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };

        self.props.update(
            message,
            State {
                inner: state,
                dirty: &mut dirty,
            },
            Context::new(context, runtime),
        );

        if dirty {
            self.set_dirty();
        }
    }

    pub fn view(&self) -> RefMut<Node<'a, M::Message>> {
        if self.dirty() {
            let mut tracker = unsafe {
                self.state
                    .borrow_mut()
                    .as_mut()
                    .map(|s| (*s) as *mut ManagedState)
                    .unwrap_or(null_mut())
                    .as_mut()
                    .unwrap()
                    .tracker()
            };

            let state = tracker.begin(0, || (self.props.mount(), Runtime::default()));
            self.component_state.set(state as *mut _);

            let mut root = unsafe { (self.props.as_ref() as *const M).as_ref().unwrap() }.view(&state.0);
            let mut query = self.style_query.clone().unwrap();
            root.acquire_state(&mut tracker);
            root.style(&mut query, self.style_position.unwrap());

            self.view.replace(Some(root));
        }
        RefMut::map(self.view.borrow_mut(), |b| b.as_mut().unwrap())
    }

    pub(crate) fn needs_poll(&self) -> bool {
        let (_, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };
        runtime.modified
    }
}

impl<'a, M: 'a + Component> GenericNode<'a, M::Output> for ComponentNode<'a, M> {
    fn get_key(&self) -> u64 {
        self.key
    }

    fn set_key(&mut self, key: u64) {
        self.key = key;
    }

    fn set_class(&mut self, _: &'a str) {}

    fn acquire_state(&mut self, tracker: &mut ManagedStateTracker<'a>) {
        self.state
            .replace(Some(tracker.begin::<ManagedState, _>(self.key, ManagedState::default)));
        tracker.end();
    }

    fn size(&self) -> (Size, Size) {
        self.view().size()
    }

    fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32) -> bool {
        self.view().hit(layout, clip, x, y)
    }

    fn focused(&self) -> bool {
        self.view().focused()
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        self.view().draw(layout, clip)
    }

    fn style(&mut self, query: &mut Query, position: (usize, usize)) {
        self.set_dirty();
        self.style_query = Some(query.clone());
        self.style_position = Some(position);
    }

    fn add_matches(&mut self, query: &mut Query) {
        self.view().add_matches(query)
    }

    fn remove_matches(&mut self, query: &mut Query) {
        self.view().remove_matches(query)
    }

    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        event: Event,
        context: &mut WidgetContext<<M as Component>::Output>,
    ) {
        let mut sub_context = context.sub_context();
        self.view().event(layout, clip, event, &mut sub_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }

        for message in sub_context {
            self.update(message, context);
        }

        let (_, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };
        while runtime.modified {
            for message in runtime.poll(&mut context.task_context()) {
                self.update(message, context);
            }
        }
    }

    fn poll(&mut self, context: &mut WidgetContext<<M as Component>::Output>) {
        let mut sub_context = context.sub_context();
        self.view().poll(&mut sub_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }

        for message in sub_context {
            self.update(message, context);
        }

        let (_, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };
        loop {
            for message in runtime.poll(&mut context.task_context()) {
                self.update(message, context);
            }
            if !runtime.modified {
                break;
            }
        }
    }
}

unsafe impl<'a, M: 'a + Component> Send for ComponentNode<'a, M> {}

impl<'a, M: 'a + Component> Drop for ComponentNode<'a, M> {
    fn drop(&mut self) {
        self.view.replace(None);
    }
}

impl<Message> Default for Runtime<Message> {
    fn default() -> Self {
        Self {
            futures: Vec::new(),
            streams: Vec::new(),
            modified: false,
        }
    }
}

impl<Message> Runtime<Message> {
    pub fn wait<F: 'static + Future<Output = Message> + Send + Sync + Unpin>(&mut self, fut: F) {
        self.futures.push(Box::new(fut));
        self.modified = true;
    }

    pub fn stream<S: 'static + Stream<Item = Message> + Send + Sync + Unpin>(&mut self, stream: S) {
        self.streams.push(Box::new(stream));
        self.modified = true;
    }

    pub(crate) fn poll(&mut self, cx: &mut std::task::Context) -> Vec<Message> {
        self.modified = false;

        let mut result = Vec::new();

        let mut i = 0;
        while i < self.futures.len() {
            match self.futures[i].poll_unpin(&mut *cx) {
                Poll::Ready(message) => {
                    result.push(message);
                    drop(self.futures.remove(i));
                }
                Poll::Pending => {
                    i += 1;
                }
            }
        }

        let mut i = 0;
        while i < self.streams.len() {
            match self.streams[i].poll_next_unpin(&mut *cx) {
                Poll::Ready(Some(message)) => {
                    result.push(message);
                }
                Poll::Ready(None) => {
                    drop(self.streams.remove(i));
                }
                Poll::Pending => {
                    i += 1;
                }
            }
        }

        result
    }
}

impl<'a, T> Deref for State<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner
    }
}

impl<'a, T> DerefMut for State<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        *self.dirty = true;
        self.inner
    }
}

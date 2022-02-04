use std::cell::{Cell, RefCell, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::ptr::null_mut;
use std::task::Poll;

use futures::{FutureExt, Stream, StreamExt};

use crate::bitset::BitSet;
use crate::component::Component;
use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, Node};
use crate::style::tree::Query;
use crate::tracker::{ManagedState, ManagedStateTracker};
use crate::widget::Context;

pub struct ComponentNode<'a, C: 'a + Component> {
    props: Box<C>,
    state: RefCell<Option<&'a mut ManagedState>>,
    view: RefCell<Option<Node<'a, C::Message>>>,
    component_state: Cell<*mut (C::State, Runtime<C::Message>)>,
    style_query: Option<Query>,
    style_position: (usize, usize),
    style_matches: BitSet,
    key: u64,
    waker: Option<std::task::Waker>,
}

/// Runtime for submitting future messages to [`Component::update`](../component/trait.Component.html#method.update).
pub struct Runtime<Message> {
    futures: Vec<Pin<Box<dyn Future<Output = Message> + Send + Sync>>>,
    streams: Vec<Pin<Box<dyn Stream<Item = Message> + Send + Sync>>>,
    waker: Option<std::task::Waker>,
}

/// Mutable state accessor.
/// By wrapping the mutable reference, the runtime knows if the state was mutated and the view should be refreshed.
pub struct DetectMut<'a, T> {
    inner: &'a mut T,
    dirty: &'a mut bool,
}

impl<'a, C: 'a + Component> ComponentNode<'a, C> {
    pub fn new(props: C) -> Self {
        let mut hasher = DefaultHasher::new();
        std::any::type_name::<C>().hash(&mut hasher);
        Self {
            props: Box::new(props),
            state: RefCell::new(None),
            view: RefCell::new(None),
            component_state: Cell::new(null_mut()),
            style_query: None,
            style_position: (0, 1),
            style_matches: BitSet::new(),
            key: hasher.finish(),
            waker: None,
        }
    }

    pub fn dirty(&self) -> bool {
        self.view.borrow().is_none()
    }

    pub fn set_dirty(&self) {
        self.view.replace(None);
    }

    pub fn props(&self) -> &C {
        self.props.as_ref()
    }

    pub fn props_mut(&mut self) -> &mut C {
        self.set_dirty();
        self.props.as_mut()
    }

    pub fn update(&mut self, message: C::Message, context: &mut Context<C::Output>) {
        let mut dirty = false;

        let mut component_state = self.component_state.get();
        if component_state.is_null() {
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

            component_state = tracker.begin(0, || {
                let mut runtime = Runtime {
                    futures: Vec::new(),
                    streams: Vec::new(),
                    waker: self.waker.clone(),
                };
                let state = self.props.mount(&mut runtime);
                (state, runtime)
            }) as *mut _;
            self.component_state.set(component_state);
        }

        let (state, runtime) = unsafe { component_state.as_mut().unwrap() };

        self.props.update(
            message,
            DetectMut {
                inner: state,
                dirty: &mut dirty,
            },
            runtime,
            context,
        );

        if dirty {
            self.set_dirty();
            context.redraw();
        }
    }

    pub fn view(&self) -> RefMut<Node<'a, C::Message>> {
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

            let state = tracker.begin(0, || {
                let mut runtime = Runtime {
                    futures: Vec::new(),
                    streams: Vec::new(),
                    waker: self.waker.clone(),
                };
                let state = self.props.mount(&mut runtime);
                (state, runtime)
            });
            self.component_state.set(state as *mut _);

            let mut root = unsafe { (self.props.as_ref() as *const C).as_ref().unwrap() }.view(&state.0);
            let mut query = self.style_query.clone().unwrap();
            root.acquire_state(&mut tracker);
            root.style(&mut query, self.style_position);

            if let Some(waker) = self.waker.as_ref() {
                root.acquire_waker(waker);
            }

            self.view.replace(Some(root));
        }
        RefMut::map(self.view.borrow_mut(), |b| b.as_mut().unwrap())
    }
}

impl<'a, C: 'a + Component> GenericNode<'a, C::Output> for ComponentNode<'a, C> {
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

    fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32, recursive: bool) -> bool {
        self.view().hit(layout, clip, x, y, recursive)
    }

    fn focused(&self) -> bool {
        self.view().focused()
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        self.view().draw(layout, clip)
    }

    fn style(&mut self, query: &mut Query, position: (usize, usize)) {
        self.style_matches =
            query.match_widget::<String>(C::style_scope(), "", &[], self.style_position.0, self.style_position.1);
        self.style_query = Some(Query {
            style: query.style.clone(),
            ancestors: {
                let mut a = query.ancestors.clone();
                a.push(self.style_matches.clone());
                a
            },
            siblings: Vec::new(),
        });
        self.style_position = position;

        self.set_dirty();

        query.siblings.push(self.style_matches.clone());
    }

    fn add_matches(&mut self, query: &mut Query) {
        let additions = query.match_widget::<String>(
            std::any::type_name::<C>(),
            "",
            &[],
            self.style_position.0,
            self.style_position.1,
        );

        let new_style = self.style_matches.union(&additions);
        if new_style != self.style_matches {
            self.style_matches = new_style;
        }

        query.ancestors.push(additions);
        let own_siblings = std::mem::take(&mut query.siblings);
        self.view().add_matches(query);
        query.siblings = own_siblings;
        query.siblings.push(query.ancestors.pop().unwrap());
    }

    fn remove_matches(&mut self, query: &mut Query) {
        let removals = query.match_widget::<String>(
            std::any::type_name::<C>(),
            "",
            &[],
            self.style_position.0,
            self.style_position.1,
        );

        let new_style = self.style_matches.difference(&removals);
        if new_style != self.style_matches {
            self.style_matches = new_style;
        }

        query.ancestors.push(removals);
        let own_siblings = std::mem::take(&mut query.siblings);
        self.view().remove_matches(query);
        query.siblings = own_siblings;
        query.siblings.push(query.ancestors.pop().unwrap());
    }

    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        event: Event,
        context: &mut Context<<C as Component>::Output>,
    ) {
        let mut sub_context = context.sub_context();
        self.view().event(layout, clip, event, &mut sub_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }
        if sub_context.rebuild_requested() {
            self.set_dirty();
        }

        for message in sub_context {
            self.update(message, context);
        }
    }

    fn acquire_waker(&mut self, waker: &std::task::Waker) {
        self.waker = Some(waker.clone());
    }

    fn poll(&mut self, context: &mut Context<<C as Component>::Output>, task_context: &mut std::task::Context) {
        self.waker = Some(task_context.waker().clone());

        let mut sub_context = context.sub_context();
        self.view().poll(&mut sub_context, task_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }
        if sub_context.rebuild_requested() {
            self.set_dirty();
        }

        for message in sub_context {
            self.update(message, context);
        }

        let (_, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };

        for message in runtime.poll(task_context) {
            self.update(message, context);
        }
    }
}

unsafe impl<'a, C: 'a + Component> Send for ComponentNode<'a, C> {}

impl<'a, C: 'a + Component> Drop for ComponentNode<'a, C> {
    fn drop(&mut self) {
        self.view.replace(None);
    }
}

impl<Message> Runtime<Message> {
    /// Submits a messsage to the component in the future.
    pub fn wait<F: 'static + Future<Output = Message> + Send + Sync>(&mut self, fut: F) {
        self.futures.push(Box::pin(fut));
        if let Some(task) = self.waker.take() {
            task.wake();
        }
    }

    /// Submits a stream of messages to the component in the future.
    pub fn stream<S: 'static + Stream<Item = Message> + Send + Sync>(&mut self, stream: S) {
        self.streams.push(Box::pin(stream));
        if let Some(task) = self.waker.take() {
            task.wake();
        }
    }

    pub(crate) fn poll(&mut self, cx: &mut std::task::Context) -> Vec<Message> {
        self.waker = Some(cx.waker().clone());

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
                Poll::Ready(Some(message)) => result.push(message),
                Poll::Ready(None) => drop(self.streams.remove(i)),
                Poll::Pending => i += 1,
            }
        }

        result
    }
}

impl<'a, T> DetectMut<'a, T> {
    /// Force the ui to be rebuilt.
    pub fn force_update(&mut self) {
        *self.dirty = true;
    }
}

impl<'a, T> Deref for DetectMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner
    }
}

impl<'a, T> DerefMut for DetectMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        *self.dirty = true;
        self.inner
    }
}

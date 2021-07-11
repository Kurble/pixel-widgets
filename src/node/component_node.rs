use std::cell::{Cell, RefCell, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ptr::null_mut;

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, Node};
use crate::stylesheet::tree::Query;
use crate::tracker::{ManagedState, ManagedStateTracker};
use crate::widget::Context;
use crate::{Component, Runtime};

pub struct ComponentNode<'a, M: 'a + Component> {
    props: Box<M>,
    state: RefCell<Option<&'a mut ManagedState>>,
    view: RefCell<Option<Node<'a, M::Message>>>,
    component_state: Cell<*mut (M::State, Runtime<M::Message>)>,
    style_query: Option<Query>,
    style_position: Option<(usize, usize)>,
    key: u64,
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

    pub fn update(&mut self, message: M::Message) -> Vec<M::Output> {
        self.set_dirty();

        let (state, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };

        self.props.update(message, state, runtime)
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
    fn key(&self) -> u64 {
        self.key
    }

    fn set_key(&mut self, key: u64) {
        self.key = key;
    }

    fn set_class(&mut self, _: &'a str) {}

    fn acquire_state(&mut self, tracker: &mut ManagedStateTracker<'a>) {
        self.state.replace(Some(
            tracker.begin::<ManagedState, _>(self.key, || ManagedState::default()),
        ));
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
        context: &mut Context<<M as Component>::Output>,
    ) {
        let mut sub_context = context.sub_context();
        self.view().event(layout, clip, event, &mut sub_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }

        for message in sub_context {
            context.extend(self.update(message));
        }

        let (_, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };
        while runtime.modified {
            for message in runtime.poll(&mut context.task_context()) {
                context.extend(self.update(message));
            }
        }
    }

    fn poll(&mut self, context: &mut Context<<M as Component>::Output>) {
        let mut sub_context = context.sub_context();
        self.view().poll(&mut sub_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }

        for message in sub_context {
            context.extend(self.update(message));
        }

        let (_, runtime) = unsafe { self.component_state.get().as_mut().unwrap() };
        loop {
            for message in runtime.poll(&mut context.task_context()) {
                context.extend(self.update(message));
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

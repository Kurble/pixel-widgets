use std::cell::{Cell, RefCell, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::ops::DerefMut;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};

use crate::draw::Primitive;
use crate::event::{Event, NodeEvent};
use crate::layout::{Rectangle, Size};
use crate::prelude::{Context, StateVec};
use crate::stylesheet::tree::Query;
use crate::stylesheet::{Style, Stylesheet};
use crate::tracker::{ManagedState, ManagedStateTracker};
use crate::widget::{ApplyStyle, GenericNode, GenericNodeEvent, Node, Widget};
use crate::Component;
use std::hash::{Hash, Hasher};

pub struct ComponentNode<'a, M: Component> {
    props: Box<M>,
    state: Option<&'a mut ManagedState>,
    view: RefCell<Option<Node<'a, M::Message>>>,
    component_state: Cell<*mut M::State>,
    style_query: Option<Query>,
    style_position: Option<(usize, usize)>,
}

impl<'a, M: Component> ComponentNode<'a, M> {
    pub fn new(props: M) -> Self {
        Self {
            props: Box::new(props),
            state: None,
            view: RefCell::new(None),
            component_state: Cell::new(null_mut()),
            style_query: None,
            style_position: None,
        }
    }

    pub fn view(&self) -> impl DerefMut<Target = Node<'a, M::Message>> {
        if self.view.is_none() {
            unsafe {
                let mut tracker = (self.state.unwrap() as *mut ManagedState).as_mut().unwrap().tracker();

                let state = tracker.get_or_default_with(0, || self.props.mount());
                self.component_state.set(state as *mut _);

                let mut root = (self.props.as_ref() as *const M).as_ref().unwrap().view(&*state);
                let mut query = self.style_query.clone().unwrap();
                root.acquire_state(&mut tracker);
                root.style(&mut query, self.style_position.unwrap());

                self.view.replace(Some(root));
            }
        }
        RefMut::map(self.view.borrow_mut(), |b| b.as_mut().unwrap())
    }
}

impl<'a, M: Component> GenericNode<'a> for ComponentNode<M> {
    fn acquire_state(&mut self, tracker: &mut ManagedStateTracker<'a>) {
        let mut hasher = DefaultHasher::new();
        std::any::type_name::<Self>().hash(&mut hasher);
        self.state = Some(tracker.get(hasher.finish()));
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
        self.style_query = Some(query.clone());
        self.style_position = Some(position);
        self.view().style(query, position);
    }

    fn add_matches(&mut self, query: &mut Query) {
        self.view().add_matches(query)
    }

    fn remove_matches(&mut self, query: &mut Query) {
        self.view().remove_matches(query)
    }
}

impl<'a, M: Component> GenericNodeEvent<'a, M::Output> for ComponentNode<'a, M> {
    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        event: Event,
        context: &mut Context<<M as Component>::Output>,
    ) {
        let mut sub_context = Context::<M::Message>::new(context.redraw_requested(), context.cursor());
        self.view().event(layout, clip, event, &mut sub_context);

        if sub_context.redraw_requested() {
            context.redraw();
        }

        for message in sub_context {
            unsafe {
                self.view.replace(None);
                context.extend(self.props.update(message, self.component_state.get().as_mut().unwrap()));
            }
        }
    }
}

impl<'a, M: Component> Drop for ComponentNode<'a, M> {
    fn drop(&mut self) {
        self.view.replace(None);
    }
}

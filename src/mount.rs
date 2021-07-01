use crate::draw::Primitive;
use crate::event::{Event, NodeEvent};
use crate::layout::{Rectangle, Size};
use crate::prelude::{Context, StateVec};
use crate::stylesheet::tree::Query;
use crate::stylesheet::{Style, Stylesheet};
use crate::tracker::{ManagedState, ManagedStateTracker};
use crate::widget::{ApplyStyle, GenericNode, Node, Widget};
use crate::Component;
use std::cell::{RefCell, RefMut};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub struct Mount<'a, M: Component> {
    props: Box<M>,
    view: RefCell<Option<Box<dyn Widget<'static, M::Message>>>>,
    state: Option<&'a mut ManagedState>,
}

impl<M: Component, S: AsMut<M::State>> Mount<M, S> {
    pub fn new(props: M, state: S) -> Self {
        Self {
            props: Box::new(props),
            state,
            view: RefCell::new(None),
        }
    }

    pub fn state(&self) -> &M::State {
        self.state.as_ref()
    }

    pub fn state_mut(&mut self) -> &mut M::State {
        self.view.replace(None);
        self.state.as_mut()
    }

    pub fn view(&self) -> impl DerefMut<Target = dyn Widget<'static, <M as Component>::Message>> {
        if self.view.is_none() {
            unsafe {
                let mut tracker = self.state.unwrap().tracker();
                let state = tracker.get_or_default_with(0, || self.props.mount());

                let mut root = (self.props.as_ref() as *const M).as_ref().unwrap().view(state);
                //root.

                self.view.replace(Some(root));
            }
        }
        RefMut::map(self.view.borrow_mut(), |b| b.as_mut().unwrap())
    }
}

impl<'a, M: Component> GenericNode<'a> for Mount<M> {
    fn acquire_state(&mut self, tracker: &mut ManagedStateTracker<'a>) {
        self.state = Some(tracker.get(0));
    }

    fn size(&self) -> (Size, Size) {
        todo!()
    }

    fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32) -> bool {
        todo!()
    }

    fn focused(&self) -> bool {
        todo!()
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        todo!()
    }

    fn style(&mut self, query: &mut Query, position: (usize, usize)) {
        todo!()
    }

    fn add_matches(&mut self, query: &mut Query) {
        todo!()
    }

    fn remove_matches(&mut self, query: &mut Query) {
        todo!()
    }
}

impl<M: Component, S> Drop for Mount<M, S> {
    fn drop(&mut self) {
        self.view.replace(None);
    }
}

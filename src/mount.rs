use crate::draw::Primitive;
use crate::event::{Event, NodeEvent};
use crate::layout::{Rectangle, Size};
use crate::prelude::{Context, StateVec};
use crate::stylesheet::tree::Query;
use crate::stylesheet::{Style, Stylesheet};
use crate::widget::{ApplyStyle, Node, Widget};
use crate::Component;
use std::cell::{RefCell, RefMut};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub struct Mount<M: Component, S> {
    props: Box<M>,
    view: RefCell<Option<Box<dyn Widget<'static, M::Message>>>>,
    state: S,
    style: Arc<Style>,
}

impl<M: Component, S: AsMut<M::State>> Mount<M, S> {
    pub fn new(props: M, state: S, style: Arc<Style>) -> Self {
        Self {
            props: Box::new(props),
            state,
            view: RefCell::new(None),
            style,
        }
    }

    pub fn set_dirty(&mut self) {
        self.view.replace(None);
    }

    pub fn dirty(&self) -> bool {
        self.view.borrow().is_none()
    }

    pub fn model(&self) -> &M {
        &self.props
    }

    pub fn model_mut(&mut self) -> &mut M {
        self.view.replace(None);
        &mut self.props
    }

    pub fn view(&self) -> impl DerefMut<Target = dyn Widget<'static, <M as Component>::Message>> {
        if self.view.is_none() {
            unsafe {
                let mut root = (self.props.as_ref() as *const M)
                    .as_ref()
                    .unwrap()
                    .view(self.state.as_ref());
                self.view.replace(Some(root));
            }
        }
        RefMut::map(self.view.borrow_mut(), |b| b.as_mut().unwrap())
    }
}

impl<M: Component, S: AsMut<M::State>> Widget<M::Output> for Mount<M, S> {
    fn widget(&self) -> &'static str {
        self.view().widget()
    }

    fn state(&self) -> StateVec {
        self.view().state()
    }

    fn len(&self) -> usize {
        self.view().len()
    }

    fn is_empty(&self) -> bool {
        self.view().is_empty()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn ApplyStyle)) {
        self.view().visit_children(visitor)
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        self.view().size(style)
    }

    fn hit(&self, layout: Rectangle, clip: Rectangle, style: &Stylesheet, x: f32, y: f32) -> bool {
        self.view().hit(layout, clip, style, x, y)
    }

    fn focused(&self) -> bool {
        self.view().focused()
    }

    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<M::Output>,
    ) {
        let mut sub_context = Context::<M::Message>::new(context.redraw_requested(), context.cursor());
        self.view().event(layout, clip, style, event, &mut sub_context);
        if sub_context.redraw_requested() {
            context.redraw();
        }
        for message in sub_context {
            context.extend(self.model_mut().update(message, self.state.as_mut()));
        }
    }

    fn node_event(
        &mut self,
        layout: Rectangle,
        style: &Stylesheet,
        event: NodeEvent,
        context: &mut Context<M::Output>,
    ) {
        let mut sub_context = Context::<M::Message>::new(context.redraw_requested(), context.cursor());
        self.view().node_event(layout, style, event, &mut sub_context);
        if sub_context.redraw_requested() {
            context.redraw();
        }
        for message in sub_context {
            context.extend(self.model_mut().update(message, self.state.as_mut()));
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive> {
        self.view().draw(layout, clip, style)
    }
}

impl<M: Component, S> Drop for Mount<M, S> {
    fn drop(&mut self) {
        self.view.replace(None);
    }
}

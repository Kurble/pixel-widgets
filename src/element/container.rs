use crate::draw::Primitive;
use crate::element::{Element, Node, Stylable, IntoNode};
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::Model;

/// Container element for embedding a `Model` in a more general user interface.
pub struct Container<'a, M: Model, F> {
    model: &'a mut M,
    // view is a mutable borrow of `self.model`, so we have a double mutable borrow.
    // the implementation of the container makes sure that the model is never accessed during it's lifetime.
    view: Option<Node<'a, M::Message>>,
    messages: Vec<M::Message>,
    translate: F,
}

impl<'a, T, M, F> Container<'a, M, F>
where
    M: Model,
    F: Fn(&M::Message) -> Option<T>,
{
    pub fn new(model: &'a mut M, translate: F) -> Self {
        let view = unsafe { Some((model as *mut M).as_mut().unwrap().view()) };
        Self {
            model,
            view,
            messages: Vec::new(),
            translate,
        }
    }
}

impl<'a, T, M, F> Element<'a, T> for Container<'a, M, F>
where
    M: Model,
    F: Fn(&M::Message) -> Option<T>,
{
    fn element(&self) -> &'static str {
        "container"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn Stylable<'a>)) {
        visitor(self.view.as_mut().unwrap());
    }

    fn size(&self, _: &Stylesheet) -> (Size, Size) {
        self.view.as_ref().unwrap().size()
    }

    fn event(&mut self, layout: Rectangle, _: &Stylesheet, event: Event, clip: Rectangle) -> Option<T> {
        let message = self.view.as_mut().unwrap().event(layout, event, clip)?;

        if let Some(message) = (self.translate)(&message) {
            Some(message)
        } else {
            self.messages.push(message);
            None
        }
    }

    fn render(&mut self, layout: Rectangle, _: &Stylesheet) -> Vec<Primitive<'a>> {
        self.view.as_mut().unwrap().render(layout)
    }
}

impl<'a, T: 'a, M: 'a, F: 'a> IntoNode<'a, T> for Container<'a, M, F>
where
    M: Model,
    F: Fn(&M::Message) -> Option<T>,
{
}

impl<'a, M: Model, F> Drop for Container<'a, M, F> {
    fn drop(&mut self) {
        // make sure the view is dropped before we modify the model
        self.view.take();
        for message in self.messages.drain(..) {
            self.model.update(message);
        }
    }
}

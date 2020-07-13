use std::cell::Cell;
use std::rc::Rc;

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::stylesheet::*;

pub use self::button::*;
pub use self::column::*;
pub use self::input::*;
pub use self::scroll::*;
pub use self::space::*;
pub use self::text::*;
pub use self::toggle::*;
use std::borrow::Cow;
use std::ops::Deref;

pub mod button;
pub mod column;
pub mod input;
pub mod scroll;
pub mod space;
pub mod text;
pub mod toggle;

pub trait Element<'a, Message> {
    fn element(&self) -> &'static str;

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, Message>));

    fn size(&self, style: &Stylesheet) -> (Size, Size);

    fn event(&mut self, layout: Rectangle, style: &Stylesheet, event: Event) -> Option<Message>;

    fn render(&mut self, layout: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>>;
}

pub trait IntoNode<'a, Message: 'a>: 'a + Sized + Element<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        Node {
            element: Box::new(self),
            size_cache: Cell::new(None),
            style: None,
            class: None,
        }
    }

    fn class(self, class: &'a str) -> Node<'a, Message> {
        self.into_node().class(class)
    }
}

pub struct Node<'a, Message> {
    element: Box<dyn Element<'a, Message> + 'a>,
    size_cache: Cell<Option<(Size, Size)>>,
    style: Option<Rc<Stylesheet>>,
    class: Option<&'a str>,
}

impl<'a, Message> Node<'a, Message> {
    pub fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    pub fn style(&mut self, engine: &mut Style, query: &mut Query<'a>) {
        query.elements.push(self.element.element());
        if let Some(class) = self.class {
            query.classes.push(Cow::Borrowed(class));
        }

        self.style.replace(engine.get(query));
        self.element
            .visit_children(&mut |child| child.style(&mut *engine, &mut *query));

        query.elements.pop();
        if self.class.is_some() {
            query.classes.pop();
        }
    }

    pub fn size(&self) -> (Size, Size) {
        if self.size_cache.get().is_none() {
            let stylesheet = self.style.as_ref().unwrap().deref();
            self.size_cache.replace(Some(self.element.size(stylesheet)));
        }
        self.size_cache.get().unwrap()
    }

    pub fn event(&mut self, layout: Rectangle, event: Event) -> Option<Message> {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.event(layout, stylesheet, event)
    }

    pub fn render(&mut self, layout: Rectangle) -> Vec<Primitive<'a>> {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.render(layout, stylesheet)
    }
}

impl<'a, Message: 'a> Element<'a, Message> for Node<'a, Message> {
    fn element(&self) -> &'static str {
        panic!("element methods should not be called directly on Node")
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, Message>)) {
        panic!("element methods should not be called directly on Node")
    }

    fn size(&self, _: &Stylesheet) -> (Size, Size) {
        panic!("element methods should not be called directly on Node")
    }

    fn event(&mut self, _: Rectangle, _: &Stylesheet, _: Event) -> Option<Message> {
        panic!("element methods should not be called directly on Node")
    }

    fn render(&mut self, _: Rectangle, _: &Stylesheet) -> Vec<Primitive<'a>> {
        panic!("element methods should not be called directly on Node")
    }
}

impl<'a, Message: 'a> IntoNode<'a, Message> for Node<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        self
    }
}

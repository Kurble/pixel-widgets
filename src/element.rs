use std::cell::Cell;

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::stylesheet::Stylesheet;

pub use self::button::*;
pub use self::column::*;
pub use self::space::*;
pub use self::text::*;

pub mod button;
pub mod column;
pub mod space;
pub mod text;

pub trait Element<'a, Message> {
    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size);

    fn event(&mut self, layout: Rectangle, stylesheet: &Stylesheet, event: Event) -> Option<Message>;

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>>;
}

pub trait IntoNode<'a, Message: 'a>: 'a + Sized + Element<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        Node {
            element: Box::new(self),
            size_cache: Cell::new(None),
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
    class: Option<&'a str>,
}

impl<'a, Message> Node<'a, Message> {
    pub fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    fn stylesheet<'s>(&self, stylesheet: &'s Stylesheet) -> &'s Stylesheet {
        if let Some(class) = self.class {
            if let Some(stylesheet) = stylesheet.classes.get(class) {
                stylesheet
            } else {
                stylesheet
            }
        } else {
            stylesheet
        }
    }
}

impl<'a, Message> Element<'a, Message> for Node<'a, Message> {
    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        if self.size_cache.get().is_none() {
            let stylesheet = self.stylesheet(stylesheet);
            self.size_cache.replace(Some(self.element.size(stylesheet)));
        }
        self.size_cache.get().unwrap()
    }

    fn event(&mut self, layout: Rectangle, stylesheet: &Stylesheet, event: Event) -> Option<Message> {
        let stylesheet = self.stylesheet(stylesheet);
        self.element.event(layout, stylesheet, event)
    }

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let stylesheet = self.stylesheet(stylesheet);
        self.element.render(layout, stylesheet)
    }
}

impl<'a, Message: 'a> IntoNode<'a, Message> for Node<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        self
    }
}

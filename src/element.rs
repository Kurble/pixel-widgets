use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

use crate::Context;
use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::stylesheet::*;

pub use self::button::Button;
pub use self::column::Column;
pub use self::input::Input;
pub use self::layers::Layers;
pub use self::row::Row;
pub use self::scroll::Scroll;
pub use self::space::Space;
pub use self::text::Text;
pub use self::toggle::Toggle;
pub use self::window::Window;

pub mod button;
pub mod column;
pub mod input;
pub mod layers;
pub mod row;
pub mod scroll;
pub mod space;
pub mod text;
pub mod toggle;
pub mod window;

pub trait Element<'a, Message> {
    fn element(&self) -> &'static str;

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn Stylable<'a>));

    fn size(&self, style: &Stylesheet) -> (Size, Size);

    fn hit(&self, layout: Rectangle, clip: Rectangle, _style: &Stylesheet, x: f32, y: f32) -> bool {
        layout.point_inside(x, y) && clip.point_inside(x, y)
    }

    fn event(
        &mut self,
        _layout: Rectangle,
        _clip: Rectangle,
        _style: &Stylesheet,
        _event: Event,
        _context: &mut Context<Message>,
    ) {
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>>;
}

pub trait IntoNode<'a, Message: 'a>: 'a + Sized {
    fn into_node(self) -> Node<'a, Message>;

    fn class(self, class: &'a str) -> Node<'a, Message> {
        self.into_node().class(class)
    }
}

pub trait Stylable<'a> {
    fn style(&mut self, engine: &mut Style, query: &mut Query<'a>);
}

pub struct Node<'a, Message> {
    element: Box<dyn Element<'a, Message> + 'a>,
    size_cache: Cell<Option<(Size, Size)>>,
    style: Option<Rc<Stylesheet>>,
    class: Option<&'a str>,
}

impl<'a, Message> Node<'a, Message> {
    pub fn new<T: 'a + Element<'a, Message>>(element: T) -> Self {
        Node {
            element: Box::new(element),
            size_cache: Cell::new(None),
            style: None,
            class: None,
        }
    }

    pub fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    pub fn size(&self) -> (Size, Size) {
        if self.size_cache.get().is_none() {
            let stylesheet = self.style.as_ref().unwrap().deref();
            self.size_cache.replace(Some(self.element.size(stylesheet)));
        }
        self.size_cache.get().unwrap()
    }

    pub fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32) -> bool {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.hit(layout, clip, stylesheet, x, y)
    }

    pub fn event(&mut self, layout: Rectangle, clip: Rectangle, event: Event, context: &mut Context<Message>) {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.event(layout, clip, stylesheet, event, context);
    }

    pub fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.draw(layout, clip, stylesheet)
    }
}

impl<'a, Message: 'a> IntoNode<'a, Message> for Node<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        self
    }
}

impl<'a, Message: 'a> Stylable<'a> for Node<'a, Message> {
    fn style(&mut self, engine: &mut Style, query: &mut Query<'a>) {
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
}

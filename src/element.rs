use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;

pub use self::panel::*;

pub mod panel;

pub trait Element<'a, Message> {
    fn size(&self) -> (Size, Size);

    fn event(&mut self, layout: Rectangle, event: Event) -> Option<Message>;

    fn render(&mut self, layout: Rectangle) -> Vec<Primitive>;
}


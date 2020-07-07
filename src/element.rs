use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::stylesheet::Stylesheet;

pub use self::space::*;
pub use self::text::*;

pub mod space;
pub mod text;

pub trait Element<'a, Message> {
    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size);

    fn event(&mut self, layout: Rectangle, stylesheet: &Stylesheet, event: Event) -> Option<Message>;

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>>;
}


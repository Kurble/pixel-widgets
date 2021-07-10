pub use crate::draw::Image;
use crate::draw::Primitive;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::Widget;

impl<'a, T: 'a> Widget<'a, T> for &'a Image {
    type State = ();

    fn mount(&self) -> Self::State {
        ()
    }

    fn widget(&self) -> &'static str {
        "image"
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        let width = match style.width {
            Size::Shrink => Size::Exact(self.size.width()),
            other => other,
        };
        let height = match style.height {
            Size::Shrink => Size::Exact(self.size.height()),
            other => other,
        };
        (width, height)
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        vec![Primitive::DrawImage(self.clone(), layout, style.color)]
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for &'a Image {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

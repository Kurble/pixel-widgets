pub use crate::draw::Image;
use crate::draw::Primitive;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{IntoNode, Node, Widget};

impl<'a, T: 'a> Widget<'a, T> for &'a Image {
    fn widget(&self) -> &'static str {
        "image"
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, T>)) {}

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
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

    fn draw(&mut self, layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        vec![Primitive::DrawImage(self.clone(), layout, style.color)]
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for &'a Image {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

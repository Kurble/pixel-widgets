use crate::draw::*;
use crate::layout::{Rectangle, Size};
use crate::node::{IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::*;

/// Empty widget
pub struct Space;

impl<'a, T> Widget<'a, T> for Space {
    type State = ();

    fn mount(&self) -> Self::State {
        ()
    }

    fn widget(&self) -> &'static str {
        "space"
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        style.background.resolve_size(
            (style.width, style.height),
            (Size::Exact(0.0), Size::Exact(0.0)),
            style.padding,
        )
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, _clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        style.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Space {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

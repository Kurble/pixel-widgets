use crate::widget::{Widget, Node, IntoNode};
use crate::draw::Primitive;
use crate::layout::{Size, Rectangle};
use crate::stylesheet::Stylesheet;

/// Dummy widget that has a custom widget name
pub struct Dummy {
    widget: &'static str,
}

impl Dummy {
    /// Construct a new `Dummy` with a widget name
    pub fn new(widget: &'static str) -> Self {
        Self { widget }
    }
}

impl<'a, T: 'a> Widget<'a, T> for Dummy {
    fn widget(&self) -> &'static str { self.widget }

    fn len(&self) -> usize { 0 }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, T>)) { }

    fn size(&self, style: &Stylesheet) -> (Size, Size) { (style.width, style.height) }

    fn draw(&mut self, layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        style.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Dummy {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

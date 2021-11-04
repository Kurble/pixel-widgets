use crate::draw::Primitive;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::style::Stylesheet;
use crate::widget::Widget;

/// Dummy widget that has a custom widget name
pub struct Dummy {
    widget: &'static str,
}

impl Dummy {
    /// Construct a new `Dummy` with a widget name
    pub fn new(widget: &'static str) -> Self {
        Self { widget }
    }

    /// Sets the widget name for this dummy
    pub fn widget(mut self, widget: &'static str) -> Self {
        self.widget = widget;
        self
    }
}

impl Default for Dummy {
    fn default() -> Self {
        Dummy { widget: "" }
    }
}

impl<'a, T: 'a> Widget<'a, T> for Dummy {
    type State = ();

    fn mount(&self) {}

    fn widget(&self) -> &'static str {
        self.widget
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        style.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Dummy {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

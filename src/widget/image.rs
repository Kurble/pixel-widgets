pub use crate::draw::ImageData;
use crate::draw::Primitive;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::Widget;
use std::marker::PhantomData;

pub struct Image<'a>(*const ImageData, PhantomData<&'a ()>);

impl<'a> Image<'a> {
    pub fn image(mut self, image: &'a ImageData) -> Self {
        self.0 = image as _;
        self
    }

    fn content(&self) -> &ImageData {
        unsafe { self.0.as_ref().expect("image of `Image` must be set") }
    }
}

impl<'a> Default for Image<'a> {
    fn default() -> Self {
        Self(std::ptr::null(), PhantomData)
    }
}

unsafe impl<'a> Send for Image<'a> {}

impl<'a, T: 'a> Widget<'a, T> for Image<'a> {
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
            Size::Shrink => Size::Exact(self.content().size.width()),
            other => other,
        };
        let height = match style.height {
            Size::Shrink => Size::Exact(self.content().size.height()),
            other => other,
        };
        (width, height)
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        vec![Primitive::DrawImage(self.content().clone(), layout, style.color)]
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Image<'a> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

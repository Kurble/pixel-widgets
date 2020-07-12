use super::{Element, Node};
use crate::draw::Primitive;
use crate::element::IntoNode;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

pub struct Column<'a, T> {
    children: Vec<Node<'a, T>>,
    layout: Vec<Rectangle>,
}

impl<'a, T: 'a> Column<'a, T> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            layout: Vec::new(),
        }
    }

    pub fn push<I: IntoNode<'a, T> + 'a>(mut self, item: I) -> Self {
        self.children.push(item.into_node());
        self
    }

    fn layout(
        &mut self,
        layout: Rectangle,
        stylesheet: &Stylesheet,
    ) -> impl Iterator<Item = (&mut Node<'a, T>, Rectangle)> {
        if self.layout.len() != self.children.len() {
            let align = stylesheet.align_horizontal;
            let mut available_parts = self.children.iter().map(|c| c.size().1.parts()).sum();
            let mut available_space = layout.height();
            let mut cursor = 0.0;
            self.layout = self
                .children
                .iter()
                .map(|child| {
                    let (w, h) = child.size();
                    let parts = h.parts();
                    let w = w.resolve(layout.width(), w.parts());
                    let h = h.resolve(available_space, available_parts);
                    let x = align.resolve_start(w, layout.width());
                    let y = cursor;

                    cursor += h;
                    available_space -= h;
                    available_parts -= parts;
                    Rectangle::from_xywh(x, y, w, h).translate(layout.left, layout.top)
                })
                .collect();
        }
        self.children.iter_mut().zip(self.layout.iter().cloned())
    }
}

impl<'a, T: 'a> Element<'a, T> for Column<'a, T> {
    fn element(&self) -> &'static str {
        "column"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        self.children.iter_mut().for_each(visitor);
    }

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        let width = match stylesheet.width {
            Size::Shrink => Size::Exact(
                self.children
                    .iter()
                    .fold(0.0, |size, child| match child.size().0 {
                        Size::Exact(child_size) => size.max(child_size),
                        _ => size,
                    }),
            ),
            other => other,
        };
        let height = match stylesheet.height {
            Size::Shrink => Size::Exact(
                self.children
                    .iter()
                    .fold(0.0, |size, child| match child.size().1 {
                        Size::Exact(child_size) => size + child_size,
                        _ => size,
                    }),
            ),
            other => other,
        };
        (width, height)
    }

    fn event(&mut self, layout: Rectangle, stylesheet: &Stylesheet, event: Event) -> Option<T> {
        let mut result = None;
        for (child, layout) in self.layout(layout, stylesheet) {
            result = result.or(child.event(layout, event));
        }
        result
    }

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();

        result = self
            .layout(layout, stylesheet)
            .fold(result, |mut result, (child, layout)| {
                result.append(&mut child.render(layout));
                result
            });

        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Column<'a, T> {}

use super::{Element, Node};
use crate::draw::Primitive;
use crate::element::{Context, IntoNode, Stylable};
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

/// Layout child elements vertically
pub struct Column<'a, T> {
    children: Vec<Node<'a, T>>,
    layout: Vec<Rectangle>,
}

impl<'a, T: 'a> Column<'a, T> {
    /// Construct a new Column
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            layout: Vec::new(),
        }
    }

    /// Adds a child element to the column
    pub fn push<I: IntoNode<'a, T> + 'a>(mut self, item: I) -> Self {
        self.children.push(item.into_node());
        self
    }

    /// Adds child elements using an iterator
    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, T> + 'a>(mut self, iter: I) -> Self {
        self.children.extend(iter.into_iter().map(IntoNode::into_node));
        self
    }

    fn layout(
        &mut self,
        layout: Rectangle,
        stylesheet: &Stylesheet,
    ) -> impl Iterator<Item = (&mut Node<'a, T>, Rectangle)> {
        if self.layout.len() != self.children.len() {
            let align = stylesheet.align_horizontal;
            let available_parts = self.children.iter().map(|c| c.size().1.parts()).sum();
            let available_space = layout.height() - self.children.iter().map(|c| c.size().1.min_size()).sum::<f32>();
            let mut cursor = 0.0;
            self.layout = self
                .children
                .iter()
                .map(|child| {
                    let (w, h) = child.size();
                    let w = w.resolve(layout.width(), w.parts());
                    let h = h
                        .resolve(available_space, available_parts)
                        .min(layout.height() - cursor);
                    let x = align.resolve_start(w, layout.width());
                    let y = cursor;

                    cursor += h;
                    Rectangle::from_xywh(x, y, w, h)
                })
                .collect();
        }
        self.children.iter_mut().zip(
            self.layout
                .iter()
                .map(move |relative| relative.translate(layout.left, layout.top)),
        )
    }
}

impl<'a, T: 'a> Element<'a, T> for Column<'a, T> {
    fn element(&self) -> &'static str {
        "column"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn Stylable<'a>)) {
        self.children.iter_mut().for_each(|child| visitor(child));
    }

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        let width = match stylesheet.width {
            Size::Shrink => Size::Exact(self.children.iter().fold(0.0, |size, child| match child.size().0 {
                Size::Exact(child_size) => size.max(child_size),
                _ => size,
            })),
            other => other,
        };
        let height = match stylesheet.height {
            Size::Shrink => Size::Exact(self.children.iter().fold(0.0, |size, child| match child.size().1 {
                Size::Exact(child_size) => size + child_size,
                _ => size,
            })),
            other => other,
        };
        (width, height)
    }

    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        stylesheet: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        for (child, layout) in self.layout(layout, stylesheet) {
            if let Some(clip) = clip.intersect(&layout) {
                child.event(layout, clip, event, context);
            }
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();

        result = self
            .layout(layout, stylesheet)
            .fold(result, |mut result, (child, layout)| {
                result.extend(child.draw(layout, clip));
                result
            });

        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Column<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

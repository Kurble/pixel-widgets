use super::{Widget, Node};
use crate::draw::Primitive;
use crate::widget::{Context, IntoNode};
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

/// Layout child widgets horizontally
pub struct Row<'a, T> {
    children: Vec<Node<'a, T>>,
    layout: Vec<Rectangle>,
}

impl<'a, T: 'a> Row<'a, T> {
    /// Construct a new `Row`
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            layout: Vec::new(),
        }
    }

    /// Adds a child widget to the row
    pub fn push<I: IntoNode<'a, T> + 'a>(mut self, item: I) -> Self {
        self.children.push(item.into_node());
        self
    }

    /// Adds child widgets using an iterator
    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, T> + 'a>(mut self, iter: I) -> Self {
        self.children.extend(iter.into_iter().map(IntoNode::into_node));
        self
    }

    fn layout(
        &mut self,
        layout: Rectangle,
        style: &Stylesheet,
    ) -> impl Iterator<Item = (&mut Node<'a, T>, Rectangle)> {
        let layout = style.background.content_rect(layout, style.padding);
        if self.layout.len() != self.children.len() {
            let align = style.align_vertical;
            let available_parts = self.children.iter().map(|c| c.size().0.parts()).sum();
            let available_space = layout.width() - self.children.iter().map(|c| c.size().0.min_size()).sum::<f32>();
            let mut cursor = 0.0;
            self.layout = self
                .children
                .iter()
                .map(|child| {
                    let (w, h) = child.size();
                    let w = w.resolve(available_space, available_parts).min(layout.width() - cursor);
                    let h = h.resolve(layout.height(), h.parts());
                    let x = cursor;
                    let y = align.resolve_start(h, layout.height());

                    cursor += w;
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

impl<'a, T: 'a> Widget<'a, T> for Row<'a, T> {
    fn widget(&self) -> &'static str {
        "row"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        self.children.iter_mut().for_each(|child| visitor(child));
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        let width = match style.width {
            Size::Shrink => Size::Exact(self.children.iter().fold(0.0, |size, child| match child.size().0 {
                Size::Exact(child_size) => size + child_size,
                _ => size,
            })),
            other => other,
        };
        let height = match style.height {
            Size::Shrink => Size::Exact(self.children.iter().fold(0.0, |size, child| match child.size().1 {
                Size::Exact(child_size) => size.max(child_size),
                _ => size,
            })),
            other => other,
        };
        style.background.resolve_size((style.width, style.height), (width, height), style.padding)
    }

    fn focused(&self) -> bool {
        self.children.iter().fold(false, |focused, child| focused || child.focused())
    }

    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        stylesheet: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        let focused = self.children.iter().position(|child| child.focused());

        for (index, (child, layout)) in self.layout(layout, stylesheet).enumerate() {
            if Some(index) == focused {
                child.event(layout, clip, event, context);
            } else if focused.is_none() {
                if let Some(clip) = clip.intersect(&layout) {
                    child.event(layout, clip, event, context);
                }
            }
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();

        result.extend(stylesheet.background.render(layout));

        result = self
            .layout(layout, stylesheet)
            .fold(result, |mut result, (child, layout)| {
                result.extend(child.draw(layout, clip));
                result
            });

        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Row<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

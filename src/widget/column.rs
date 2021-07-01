use super::{Node, Widget};
use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{ApplyStyle, Context, IntoNode};
use crate::tracker::ManagedStateTracker;
use std::hash::{Hash, Hasher};

/// Layout child widgets vertically
pub struct Column<'a, T> {
    children: Vec<Node<'a, T>>,
    layout: Vec<Rectangle>,
}

impl<'a, T: 'a> Column<'a, T> {
    /// Construct a new Column
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a child widget to the column
    pub fn push<I: IntoNode<'a, T> + 'a>(mut self, item: I) -> Self {
        self.children.push(item.into_node());
        self
    }

    /// Adds child widgets using an iterator
    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, T> + 'a>(mut self, iter: I) -> Self {
        self.children.extend(iter.into_iter().map(IntoNode::into_node));
        self
    }

    fn layout(&mut self, layout: Rectangle, style: &Stylesheet) -> impl Iterator<Item = (&mut Node<'a, T>, Rectangle)> {
        let layout = style.background.content_rect(layout, style.padding);
        if self.layout.len() != self.children.len() {
            let align = style.align_horizontal;
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

impl<'a, T: 'a> Default for Column<'a, T> {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            layout: Vec::new(),
        }
    }
}

impl<'a, T> Hash for Column<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "column".hash(state)
    }
}

impl<'a, T: 'a + Send> Widget<'a, T> for Column<'a, T> {
    fn widget(&self) -> &'static str {
        "column"
    }

    fn acquire_state(&mut self, _: &mut ManagedStateTracker<'a>) { }

    fn len(&self) -> usize {
        self.children.len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn ApplyStyle)) {
        self.children.iter_mut().for_each(|child| visitor(child));
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        let width = match style.width {
            Size::Shrink => Size::Exact(self.children.iter().fold(0.0, |size, child| match child.size().0 {
                Size::Exact(child_size) => size.max(child_size),
                _ => size,
            })),
            other => other,
        };
        let height = match style.height {
            Size::Shrink => Size::Exact(self.children.iter().fold(0.0, |size, child| match child.size().1 {
                Size::Exact(child_size) => size + child_size,
                _ => size,
            })),
            other => other,
        };

        style
            .background
            .resolve_size((style.width, style.height), (width, height), style.padding)
    }

    fn focused(&self) -> bool {
        self.children.iter().any(|child| child.focused())
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

impl<'a, T: 'a + Send> IntoNode<'a, T> for Column<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

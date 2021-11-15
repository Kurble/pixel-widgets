use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::style::Stylesheet;
use crate::widget::{Context, Widget};

/// The anchor from which to apply the offset of a `Panel`
#[allow(missing_docs)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// A panel with a fixed size and location within it's parent
pub struct Panel<'a, T> {
    offset: (f32, f32),
    anchor: Anchor,
    content: Option<Node<'a, T>>,
}

impl<'a, T: 'a> Panel<'a, T> {
    /// Construct a new `Panel`, with an offset from an anchor
    pub fn new(offset: (f32, f32), anchor: Anchor, content: impl IntoNode<'a, T>) -> Self {
        Self {
            offset,
            anchor,
            content: Some(content.into_node()),
        }
    }

    /// Sets the (x, y) offset from the anchor.
    pub fn offset(mut self, offset: (f32, f32)) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the anchor of the frame.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets the content widget from the first element of an iterator.
    pub fn extend<I: IntoIterator<Item = N>, N: IntoNode<'a, T> + 'a>(mut self, iter: I) -> Self {
        if self.content.is_none() {
            self.content = iter.into_iter().next().map(IntoNode::into_node);
        }
        self
    }

    fn layout(&self, layout: Rectangle) -> Option<Rectangle> {
        let (content_width, content_height) = self.content().size();
        let (h, v) = match self.anchor {
            Anchor::TopLeft => (0, 0),
            Anchor::TopCenter => (1, 0),
            Anchor::TopRight => (2, 0),
            Anchor::CenterLeft => (0, 1),
            Anchor::Center => (1, 1),
            Anchor::CenterRight => (2, 1),
            Anchor::BottomLeft => (0, 2),
            Anchor::BottomCenter => (1, 2),
            Anchor::BottomRight => (2, 2),
        };

        let h_available = match h {
            0 => (layout.left + self.offset.0, layout.right),
            1 if self.offset.0 > 0.0 => (layout.left + self.offset.0 * 2.0, layout.right),
            1 => (layout.left, layout.right + self.offset.0 * 2.0),
            _ => (layout.left, layout.right - self.offset.0),
        };

        let v_available = match v {
            0 => (layout.top + self.offset.1, layout.bottom),
            1 if self.offset.1 > 0.0 => (layout.top + self.offset.1 * 2.0, layout.bottom),
            1 => (layout.top, layout.bottom + self.offset.1 * 2.0),
            _ => (layout.top, layout.bottom - self.offset.1),
        };

        if h_available.0 < h_available.1 && v_available.0 < v_available.1 {
            let width = match content_width {
                Size::Exact(width) => width.min(h_available.1 - h_available.0),
                Size::Fill(_) => h_available.1 - h_available.0,
                Size::Shrink => 0.0,
            };
            let height = match content_height {
                Size::Exact(height) => height.min(v_available.1 - v_available.0),
                Size::Fill(_) => v_available.1 - v_available.0,
                Size::Shrink => 0.0,
            };

            let (left, right) = match h {
                0 => (h_available.0, h_available.0 + width),
                1 => (
                    (h_available.0 + h_available.1 - width) * 0.5,
                    (h_available.0 + h_available.1 + width) * 0.5,
                ),
                _ => (h_available.1 - width, h_available.1),
            };

            let (top, bottom) = match v {
                0 => (v_available.0, v_available.0 + height),
                1 => (
                    (v_available.0 + v_available.1 - height) * 0.5,
                    (v_available.0 + v_available.1 + height) * 0.5,
                ),
                _ => (v_available.1 - height, v_available.1),
            };

            Some(Rectangle {
                left,
                right,
                top,
                bottom,
            })
        } else {
            None
        }
    }

    fn content(&self) -> &Node<'a, T> {
        self.content.as_ref().expect("content of `Panel` must be set")
    }

    fn content_mut(&mut self) -> &mut Node<'a, T> {
        self.content.as_mut().expect("content of `Panel` must be set")
    }
}

impl<'a, T: 'a> Default for Panel<'a, T> {
    fn default() -> Self {
        Self {
            offset: (0.0, 0.0),
            anchor: Anchor::TopLeft,
            content: None,
        }
    }
}

impl<'a, T: 'a> Widget<'a, T> for Panel<'a, T> {
    type State = ();

    fn mount(&self) {}

    fn widget(&self) -> &'static str {
        "panel"
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        visitor(&mut **self.content_mut());
    }

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn hit(&self, _: &(), layout: Rectangle, clip: Rectangle, _: &Stylesheet, x: f32, y: f32) -> bool {
        if layout.point_inside(x, y) && clip.point_inside(x, y) {
            self.layout(layout)
                .map(|layout| layout.point_inside(x, y))
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn focused(&self, _: &()) -> bool {
        self.content().focused()
    }

    fn event(
        &mut self,
        _: &mut (),
        layout: Rectangle,
        clip: Rectangle,
        _: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        if let Some(layout) = self.layout(layout) {
            self.content_mut().event(layout, clip, event, context)
        }
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, clip: Rectangle, _: &Stylesheet) -> Vec<Primitive<'a>> {
        if let Some(layout) = self.layout(layout) {
            self.content_mut().draw(layout, clip)
        } else {
            Vec::new()
        }
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Panel<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

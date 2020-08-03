use crate::widget::{Widget, IntoNode, Node, Dummy};
use crate::draw::Primitive;
use crate::layout::{Size, Rectangle, Direction};
use crate::stylesheet::Stylesheet;

/// A progress bar that fill up according to some progress
pub struct Progress<'a, T> {
    progress: f32,
    fill: Node<'a, T>,
}

impl<'a, T: 'a> Progress<'a, T> {
    /// Construct a new `Progress` with a progress value in the range [0.0, 1.0]
    pub fn new(progress: f32) -> Self {
        Self {
            progress,
            fill: Dummy::new("progress-fill").into_node()
        }
    }
}

impl<'a, T: 'a> Widget<'a, T> for Progress<'a, T> {
    fn widget(&self) -> &'static str {
        "progress"
    }

    fn len(&self) -> usize { 1 }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        visitor(&mut self.fill);
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        let fill = layout.after_padding(style.padding);
        let fill = match style.direction {
            Direction::LeftToRight => Rectangle {
                right: fill.left + fill.width() * self.progress,
                ..fill
            },
            Direction::RightToLeft => Rectangle {
                left: fill.right - fill.width() * self.progress,
                ..fill
            },
            Direction::TopToBottom => Rectangle {
                bottom: fill.top + fill.height() * self.progress,
                ..fill
            },
            Direction::BottomToTop => Rectangle {
                top: fill.bottom - fill.height() * self.progress,
                ..fill
            },
        };
        result.extend(self.fill.draw(fill, clip));
        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Progress<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}
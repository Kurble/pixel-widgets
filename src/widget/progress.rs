use crate::draw::Primitive;
use crate::layout::{Direction, Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::{Dummy, Widget};

/// A progress bar that fill up according to some progress
/// The bar part of the progress bar can be styled by selecting the child widget `bar` of the `progress` widget.
/// Progress accepts the `clip-bar` flag for it's style. When the `clip-bar` flag is set, the bar is always rendered
/// at full size and then clipped according to the progress. When `clip-bar` is not set, the bar itself is rendered
/// with a size that matches the progress.
pub struct Progress<'a, T> {
    progress: f32,
    fill: Node<'a, T>,
}

impl<'a, T: 'a> Progress<'a, T> {
    /// Construct a new `Progress` with a progress value in the range [0.0, 1.0]
    pub fn new(progress: f32) -> Self {
        Self {
            progress,
            fill: Dummy::new("bar").into_node(),
        }
    }

    /// Sets the progress value, which should be in the range [0.0, 1.0]
    pub fn val(mut self, val: f32) -> Self {
        self.progress = val;
        self
    }
}

impl<'a, T: 'a> Default for Progress<'a, T> {
    fn default() -> Self {
        Self {
            progress: 0.0,
            fill: Dummy::new("bar").into_node(),
        }
    }
}

impl<'a, T: 'a> Widget<'a, T> for Progress<'a, T> {
    type State = ();

    fn mount(&self) {}

    fn widget(&self) -> &'static str {
        "progress"
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        visitor(&mut *self.fill);
    }

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
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

        if self.progress > 0.0 {
            if style.contains("clip-bar") {
                if let Some(clip) = clip.intersect(&fill) {
                    result.push(Primitive::PushClip(clip));
                    result.extend(self.fill.draw(layout.after_padding(style.padding), clip));
                    result.push(Primitive::PopClip);
                }
            } else {
                result.extend(self.fill.draw(fill, clip));
            }
        }

        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Progress<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

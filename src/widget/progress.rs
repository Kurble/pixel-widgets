use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Direction, Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::style::Stylesheet;
use crate::widget::{dummy::Dummy, Widget, Context};

/// A progress bar that fill up according to some progress
/// The bar part of the progress bar can be styled by selecting the child widget `bar` of the `progress` widget.
/// Progress accepts the `clip-bar` flag for it's style. When the `clip-bar` flag is set, the bar is always rendered
/// at full size and then clipped according to the progress. When `clip-bar` is not set, the bar itself is rendered
/// with a size that matches the progress.
pub struct Progress<'a, T> {
    progress: ProgressValue,
    fill: Node<'a, T>,
}

enum ProgressValue {
    Static(f32),
    Dynamic(Box<dyn 'static + Send + FnMut() -> f32>),
}

impl<'a, T: 'a> Progress<'a, T> {
    /// Construct a new `Progress` with a progress value in the range [0.0, 1.0]
    pub fn new(progress: f32) -> Self {
        Self {
            progress: ProgressValue::Static(progress),
            fill: Dummy::new("bar").into_node(),
        }
    }

    /// Sets the progress value, which should be in the range [0.0, 1.0]
    pub fn val(mut self, val: f32) -> Self {
        self.progress = ProgressValue::Static(val);
        self
    }

    /// Sets the progress value to be calculated from a function. 
    /// The function will be called every time the progress is drawn.
    /// The returned progress must be in the range [0.0, 1.0]
    pub fn val_with(mut self, val: impl 'static + Send + FnMut() -> f32) -> Self {
        self.progress = ProgressValue::Dynamic(Box::new(val));
        self
    }
}

impl<'a, T: 'a> Default for Progress<'a, T> {
    fn default() -> Self {
        Self {
            progress: ProgressValue::Static(0.0),
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

    fn hit(
        &self,
        _: &Self::State,
        _: Rectangle,
        _: Rectangle,
        _: &Stylesheet,
        _: f32,
        _: f32,
        _: bool,
    ) -> bool {
        true
    }

    fn event(
        &mut self,
        _: &mut (),
        _: Rectangle,
        _: Rectangle,
        _: &Stylesheet,
        _: Event,
        context: &mut Context<T>,
    ) {
        if let ProgressValue::Dynamic(_) = self.progress {
            context.redraw();
        }
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let progress = match &mut self.progress {
            &mut ProgressValue::Static(value) => value,
            ProgressValue::Dynamic(dynamic) => dynamic(),
        };

        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        let fill = layout.after_padding(style.padding);
        let fill = match style.direction {
            Direction::LeftToRight => Rectangle {
                right: fill.left + fill.width() * progress,
                ..fill
            },
            Direction::RightToLeft => Rectangle {
                left: fill.right - fill.width() * progress,
                ..fill
            },
            Direction::TopToBottom => Rectangle {
                bottom: fill.top + fill.height() * progress,
                ..fill
            },
            Direction::BottomToTop => Rectangle {
                top: fill.bottom - fill.height() * progress,
                ..fill
            },
        };

        if progress > 0.0 {
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

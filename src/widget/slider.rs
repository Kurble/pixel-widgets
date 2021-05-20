use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, Dummy, IntoNode, Node, Widget};

/// Select a number using a sliding handle
/// The handle can be styled using the `handle` child widget of this widget.
pub struct Slider<'a, T, F> {
    state: &'a mut State,
    scrollbar: Node<'a, T>,
    min: f32,
    max: f32,
    value: f32,
    on_slide: F,
}

/// State for [`Slider`](struct.Slider.html)
pub struct State {
    inner: InnerState,
    cursor_x: f32,
    cursor_y: f32,
}

#[derive(Clone, Copy)]
enum InnerState {
    Idle,
    Hover,
    Drag(f32),
}

impl<'a, T: 'a, F: 'a + Fn(f32) -> T> Slider<'a, T, F> {
    /// Construct a new `Slider`
    pub fn new(state: &'a mut State, min: f32, max: f32, value: f32, on_slide: F) -> Slider<'a, T, F> {
        Self {
            state,
            scrollbar: Dummy::new("handle").into_node(),
            min,
            max,
            value: value.max(min).min(max),
            on_slide: on_slide,
        }
    }

    fn scrollbar(&self, layout: Rectangle, style: &Stylesheet) -> Rectangle {
        let content = style.background.content_rect(layout, style.padding);

        let (handle_width, _) = self.scrollbar.size();
        let handle_width = match handle_width {
            Size::Shrink => content.width() * 0.1,
            Size::Exact(x) => x,
            Size::Fill(_) => content.width() * 0.1,
        };

        let mut t = (self.value - self.min) / (self.max - self.min);
        t = t.max(0.0).min(1.0);

        Rectangle {
            left: content.left + (content.width() - handle_width) * t,
            right: content.left + (content.width() - handle_width) * t + handle_width,
            ..content
        }
    }
}

impl<'a, T: 'a, F: 'a + Send + Fn(f32) -> T> Widget<'a, T> for Slider<'a, T, F> {
    fn widget(&self) -> &'static str {
        "slider"
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        visitor(&mut self.scrollbar);
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        style
            .background
            .resolve_size((style.width, style.height), self.scrollbar.size(), style.padding)
    }

    fn event(
        &mut self,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        let content_rect = style.background.content_rect(layout, style.padding);
        let bar = self.scrollbar(layout, style);

        match (event, self.state.inner) {
            (Event::Cursor(cx, cy), InnerState::Drag(x)) => {
                context.redraw();
                self.state.cursor_x = cx;
                self.state.cursor_y = cy;

                let begin = content_rect.left;
                let end = content_rect.right - bar.width();
                let next_bar_left = (cx - x).max(begin).min(end);
                let t = (next_bar_left - begin) / (end - begin);

                self.value = self.min + t * (self.max - self.min);
                context.push((self.on_slide)(self.value));
            }
            (Event::Cursor(x, y), _) => {
                self.state.cursor_x = x;
                self.state.cursor_y = y;
                if bar.point_inside(x, y) && clip.point_inside(x, y) {
                    self.state.inner = InnerState::Hover;
                } else {
                    self.state.inner = InnerState::Idle;
                }
            }
            (Event::Press(Key::LeftMouseButton), InnerState::Hover) => {
                self.state.inner = InnerState::Drag(self.state.cursor_x - bar.left);
            }
            (Event::Release(Key::LeftMouseButton), InnerState::Drag(_)) => {
                if bar.point_inside(self.state.cursor_x, self.state.cursor_y)
                    && clip.point_inside(self.state.cursor_x, self.state.cursor_y)
                {
                    self.state.inner = InnerState::Hover;
                } else {
                    self.state.inner = InnerState::Idle;
                }
            }
            _ => (),
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        result.extend(self.scrollbar.draw(self.scrollbar(layout, style), clip));
        result
    }
}

impl<'a, T: 'a, F: 'a + Send + Fn(f32) -> T> IntoNode<'a, T> for Slider<'a, T, F> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

impl Default for State {
    fn default() -> State {
        State {
            inner: InnerState::Idle,
            cursor_x: 0.0,
            cursor_y: 0.0,
        }
    }
}

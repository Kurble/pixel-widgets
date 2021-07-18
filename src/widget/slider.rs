use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, Dummy, Widget};

/// Select a number using a sliding handle
/// The handle can be styled using the `handle` child widget of this widget.
pub struct Slider<'a, T, F> {
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
    pub fn new(min: f32, max: f32, value: f32, on_slide: F) -> Slider<'a, T, F> {
        Self {
            scrollbar: Dummy::new("handle").into_node(),
            min,
            max,
            value: value.max(min).min(max),
            on_slide,
        }
    }

    /// Sets the minimum value of the slider.
    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self.value = self.value.max(min);
        self
    }

    /// Sets the maximum value of the slider.
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self.value = self.value.min(max);
        self
    }

    /// Sets the current value of the slider.
    pub fn val(mut self, value: f32) -> Self {
        self.value = value.min(self.max).min(self.min);
        self
    }

    /// Sets the on_slide callback of the slider, which is called when the value is changed.
    pub fn on_slide<N: Fn(f32) -> T>(self, on_slide: N) -> Slider<'a, T, N> {
        Slider {
            scrollbar: self.scrollbar,
            min: self.min,
            max: self.max,
            value: self.value,
            on_slide,
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

impl<'a, T: 'a> Default for Slider<'a, T, fn(f32) -> T> {
    fn default() -> Self {
        Self {
            scrollbar: Dummy::new("handle").into_node(),
            min: 0.0,
            max: 1.0,
            value: 0.0,
            on_slide: |_| panic!("on_slide of `Slider` must be set"),
        }
    }
}

impl<'a, T: 'a, F: 'a + Send + Fn(f32) -> T> Widget<'a, T> for Slider<'a, T, F> {
    type State = State;

    fn mount(&self) -> Self::State {
        State::default()
    }

    fn widget(&self) -> &'static str {
        "slider"
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        visitor(&mut *self.scrollbar);
    }

    fn size(&self, _: &State, style: &Stylesheet) -> (Size, Size) {
        style
            .background
            .resolve_size((style.width, style.height), self.scrollbar.size(), style.padding)
    }

    fn event(
        &mut self,
        state: &mut State,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        let content_rect = style.background.content_rect(layout, style.padding);
        let bar = self.scrollbar(layout, style);

        match (event, state.inner) {
            (Event::Cursor(cx, cy), InnerState::Drag(x)) => {
                context.redraw();
                state.cursor_x = cx;
                state.cursor_y = cy;

                let begin = content_rect.left;
                let end = content_rect.right - bar.width();
                let next_bar_left = (cx - x).max(begin).min(end);
                let t = (next_bar_left - begin) / (end - begin);

                self.value = self.min + t * (self.max - self.min);
                context.push((self.on_slide)(self.value));
            }
            (Event::Cursor(x, y), _) => {
                state.cursor_x = x;
                state.cursor_y = y;
                if bar.point_inside(x, y) && clip.point_inside(x, y) {
                    state.inner = InnerState::Hover;
                } else {
                    state.inner = InnerState::Idle;
                }
            }
            (Event::Press(Key::LeftMouseButton), InnerState::Hover) => {
                state.inner = InnerState::Drag(state.cursor_x - bar.left);
            }
            (Event::Release(Key::LeftMouseButton), InnerState::Drag(_)) => {
                if bar.point_inside(state.cursor_x, state.cursor_y) && clip.point_inside(state.cursor_x, state.cursor_y)
                {
                    state.inner = InnerState::Hover;
                } else {
                    state.inner = InnerState::Idle;
                }
            }
            _ => (),
        }
    }

    fn draw(&mut self, _: &mut State, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        let scrollbar = self.scrollbar(layout, style);
        result.extend(self.scrollbar.draw(scrollbar, clip));
        result
    }
}

impl<'a, T: 'a, F: 'a + Send + Fn(f32) -> T> IntoNode<'a, T> for Slider<'a, T, F> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
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

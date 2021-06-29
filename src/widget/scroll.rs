use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{ApplyStyle, Context, Dummy, IntoNode, Node, Widget};

/// View a small section of larger widget, with scrollbars.
/// The scrollbars are only rendered if the content is larger than the view in that direction.
/// The scrollbars can be styled using the `scrollbar-horizontal` and `scrollbar-vertical` child widgets of this widget.
pub struct Scroll<'a, T> {
    state: &'a mut State,
    content: Node<'a, T>,
    scrollbar_h: Node<'a, T>,
    scrollbar_v: Node<'a, T>,
}

/// State for [`Scroll`](struct.Scroll.html)
pub struct State {
    inner: InnerState,
    scroll_x: f32,
    scroll_y: f32,
    cursor_x: f32,
    cursor_y: f32,
}

#[derive(Clone, Copy)]
enum InnerState {
    Idle,
    HoverHorizontalBar,
    HoverVerticalBar,
    DragHorizontalBar(f32),
    DragVerticalBar(f32),
}

impl<'a, T: 'a> Scroll<'a, T> {
    /// Construct a new `Scroll`
    pub fn new(state: &'a mut State, content: impl IntoNode<'a, T>) -> Scroll<'a, T> {
        Self {
            state,
            content: content.into_node(),
            scrollbar_h: Dummy::new("scrollbar-horizontal").into_node(),
            scrollbar_v: Dummy::new("scrollbar-vertical").into_node(),
        }
    }

    fn scrollbars(&self, layout: Rectangle, content: Rectangle, style: &Stylesheet) -> (Rectangle, Rectangle) {
        let content_rect = style.background.content_rect(layout, style.padding);

        let vertical_rect = {
            let mut bar = Rectangle {
                left: content_rect.right,
                top: layout.top,
                right: layout.right,
                bottom: content_rect.bottom,
            };
            let handle_range = handle_range(
                bar.top,
                self.state.scroll_y,
                bar.height(),
                content.height() - content_rect.height(),
            );
            bar.top = handle_range.0;
            bar.bottom = handle_range.1;
            bar
        };

        let horizontal_rect = {
            let mut bar = Rectangle {
                left: layout.left,
                top: content_rect.bottom,
                right: content_rect.right,
                bottom: layout.bottom,
            };
            let handle_range = handle_range(
                bar.left,
                self.state.scroll_x,
                bar.width(),
                content.width() - content_rect.width(),
            );
            bar.left = handle_range.0;
            bar.right = handle_range.1;
            bar
        };

        (vertical_rect, horizontal_rect)
    }

    fn content_layout(&self, content_rect: &Rectangle) -> Rectangle {
        let content_size = self.content.size();
        Rectangle::from_xywh(
            content_rect.left - self.state.scroll_x,
            content_rect.top - self.state.scroll_y,
            content_size
                .0
                .resolve(content_rect.width(), content_size.0.parts())
                .max(content_size.0.min_size()),
            content_size
                .1
                .resolve(content_rect.height(), content_size.1.parts())
                .max(content_size.1.min_size()),
        )
    }
}

impl<'a, T: 'a> Widget<'a, T> for Scroll<'a, T> {
    fn widget(&self) -> &'static str {
        "scroll"
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn ApplyStyle)) {
        visitor(&mut self.content);
        visitor(&mut self.scrollbar_h);
        visitor(&mut self.scrollbar_v);
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        style
            .background
            .resolve_size((style.width, style.height), self.content.size(), style.padding)
    }

    fn focused(&self) -> bool {
        self.content.focused()
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
        let content_layout = self.content_layout(&content_rect);
        let (vbar, hbar) = self.scrollbars(layout, content_layout, style);

        if self.content.focused() {
            self.content.event(content_layout, content_rect, event, context);
            return;
        }

        match (event, self.state.inner) {
            (Event::Cursor(cx, cy), InnerState::DragHorizontalBar(x)) => {
                context.redraw();
                self.state.cursor_x = cx;
                self.state.cursor_y = cy;

                let bar = Rectangle {
                    left: layout.left,
                    top: content_rect.bottom,
                    right: content_rect.right,
                    bottom: layout.bottom,
                };
                self.state.scroll_x = handle_to_scroll(
                    bar.left,
                    cx - x,
                    bar.width(),
                    content_layout.width() - content_rect.width(),
                );
            }
            (Event::Cursor(cx, cy), InnerState::DragVerticalBar(y)) => {
                context.redraw();
                self.state.cursor_x = cx;
                self.state.cursor_y = cy;

                let bar = Rectangle {
                    left: content_rect.right,
                    top: layout.top,
                    right: layout.right,
                    bottom: content_rect.bottom,
                };
                self.state.scroll_y = handle_to_scroll(
                    bar.top,
                    cy - y,
                    bar.height(),
                    content_layout.height() - content_rect.height(),
                );
            }
            (Event::Cursor(x, y), _) => {
                if let Some(clip) = clip.intersect(&content_rect) {
                    self.content.event(content_layout, clip, event, context);
                }
                self.state.cursor_x = x;
                self.state.cursor_y = y;
                if hbar.point_inside(x, y) && clip.point_inside(x, y) {
                    self.state.inner = InnerState::HoverHorizontalBar;
                } else if vbar.point_inside(x, y) && clip.point_inside(x, y) {
                    self.state.inner = InnerState::HoverVerticalBar;
                } else {
                    self.state.inner = InnerState::Idle;
                }
            }
            (Event::Press(Key::LeftMouseButton), InnerState::HoverHorizontalBar) => {
                self.state.inner = InnerState::DragHorizontalBar(self.state.cursor_x - hbar.left);
            }
            (Event::Press(Key::LeftMouseButton), InnerState::HoverVerticalBar) => {
                self.state.inner = InnerState::DragVerticalBar(self.state.cursor_y - vbar.top);
            }
            (Event::Release(Key::LeftMouseButton), InnerState::DragHorizontalBar(_))
            | (Event::Release(Key::LeftMouseButton), InnerState::DragVerticalBar(_)) => {
                if hbar.point_inside(self.state.cursor_x, self.state.cursor_y)
                    && clip.point_inside(self.state.cursor_x, self.state.cursor_y)
                {
                    self.state.inner = InnerState::HoverHorizontalBar;
                } else if vbar.point_inside(self.state.cursor_x, self.state.cursor_y)
                    && clip.point_inside(self.state.cursor_x, self.state.cursor_y)
                {
                    self.state.inner = InnerState::HoverVerticalBar;
                } else {
                    self.state.inner = InnerState::Idle;
                }
            }
            (event, InnerState::Idle) => {
                if let Some(clip) = clip.intersect(&content_rect) {
                    self.content.event(content_layout, clip, event, context);
                }
            }
            _ => (),
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let content_rect = style.background.content_rect(layout, style.padding);
        let content_layout = self.content_layout(&content_rect);
        let (vbar, hbar) = self.scrollbars(layout, content_layout, style);

        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        if let Some(clip) = clip.intersect(&content_rect) {
            result.push(Primitive::PushClip(clip));
            result.extend(self.content.draw(content_layout, content_rect));
            result.push(Primitive::PopClip);
        }
        if content_layout.width() > layout.width() {
            result.extend(self.scrollbar_h.draw(hbar, clip));
        }
        if content_layout.height() > layout.height() {
            result.extend(self.scrollbar_v.draw(vbar, clip));
        }
        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Scroll<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

impl Default for State {
    fn default() -> State {
        State {
            inner: InnerState::Idle,
            scroll_x: 0.0,
            scroll_y: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
        }
    }
}

fn handle_to_scroll(offset: f32, x: f32, length: f32, content: f32) -> f32 {
    if content > 0.0 {
        let range = handle_range(offset, content, length, content);
        let pos = (x - offset) / (range.0 - offset);
        (pos * content).max(0.0).min(content).floor()
    } else {
        0.0
    }
}

fn handle_range(offset: f32, x: f32, length: f32, content: f32) -> (f32, f32) {
    if content > 0.0 {
        let size = length * (length / (length + content));
        let start = length * (x / (length + content));
        ((offset + start).floor(), (offset + start + size).floor())
    } else {
        (offset.floor(), (offset + length).floor())
    }
}

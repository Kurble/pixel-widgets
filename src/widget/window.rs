use crate::draw::*;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, IntoNode, Node, Widget};

/// A window with a title and a content widget that can be moved by dragging the title.
pub struct Window<'a, T> {
    state: &'a mut State,
    title: Node<'a, T>,
    content: Node<'a, T>,
}

/// State for [`Window`](struct.Window.html)
pub struct State {
    x: f32,
    y: f32,
    cursor_x: f32,
    cursor_y: f32,
    inner: InnerState,
}

#[derive(Clone, Copy)]
enum InnerState {
    Idle,
    Dragging(f32, f32),
}

impl<'a, T: 'a> Window<'a, T> {
    /// Constructs a new `Window`
    pub fn new(state: &'a mut State, title: impl IntoNode<'a, T>, content: impl IntoNode<'a, T>) -> Self {
        Self {
            state,
            title: title.into_node(),
            content: content.into_node(),
        }
    }

    fn layout(&self, viewport: Rectangle, style: &Stylesheet) -> (Rectangle, Rectangle, Rectangle) {
        let title_size = self.title.size();
        let title_width = title_size.0.min_size();
        let title_height = title_size.1.min_size();
        let content_size = self.content.size();
        let content_width = content_size.0.min_size();
        let content_height = content_size.1.min_size();
        let width = title_width.max(content_width);
        let height = title_height + content_height;
        let padding = style.background.padding();
        let padding = Rectangle {
            left: padding.left + style.padding.left,
            right: padding.right + style.padding.right,
            top: padding.top + style.padding.top,
            bottom: padding.bottom + style.padding.bottom,
        };
        let layout = Rectangle::from_xywh(
            viewport.left + self.state.x,
            viewport.top + self.state.y,
            width + padding.left + padding.right,
            height + padding.top + padding.bottom,
        );
        let title_content = layout.after_padding(padding);
        let title = Rectangle::from_xywh(
            title_content.left,
            title_content.top,
            title_size.0.resolve(title_content.width(), title_size.0.parts()),
            title_height,
        );
        let content = Rectangle::from_xywh(
            title_content.left,
            title_content.top + title_height,
            content_size.0.resolve(title_content.width(), content_size.0.parts()),
            content_height,
        );
        let align = |rect: Rectangle| {
            rect.translate(
                style
                    .align_horizontal
                    .resolve_start(rect.width(), title_content.width()),
                0.0,
            )
        };
        (layout, align(title), align(content))
    }
}

impl<'a, T: 'a> Widget<'a, T> for Window<'a, T> {
    fn widget(&self) -> &'static str {
        "window"
    }

    fn len(&self) -> usize {
        2
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        visitor(&mut self.title);
        visitor(&mut self.content);
    }

    fn size(&self, _: &Stylesheet) -> (Size, Size) {
        (Size::Fill(1), Size::Fill(1))
    }

    fn hit(&self, viewport: Rectangle, clip: Rectangle, style: &Stylesheet, x: f32, y: f32) -> bool {
        if clip.point_inside(x, y) {
            let (layout, _, _) = self.layout(viewport, style);
            layout.point_inside(x, y)
        } else {
            false
        }
    }

    fn focused(&self) -> bool {
        self.title.focused() || self.content.focused()
    }

    fn event(
        &mut self,
        viewport: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        let (layout, title, content) = self.layout(viewport, style);

        if self.title.focused() {
            self.title.event(title, clip, event, context);
            return;
        }

        if self.content.focused() {
            self.content.event(content, clip, event, context);
            return;
        }

        match (event, self.state.inner) {
            (Event::Cursor(x, y), InnerState::Idle) => {
                self.state.cursor_x = x;
                self.state.cursor_y = y;
            }

            (Event::Press(Key::LeftMouseButton), InnerState::Idle) => {
                if clip.point_inside(self.state.cursor_x, self.state.cursor_y)
                    && title.point_inside(self.state.cursor_x, self.state.cursor_y)
                {
                    context.redraw();
                    self.state.inner =
                        InnerState::Dragging(self.state.cursor_x - layout.left, self.state.cursor_y - layout.top);
                }
            }

            (Event::Cursor(x, y), InnerState::Dragging(anchor_x, anchor_y)) => {
                context.redraw();
                self.state.cursor_x = x;
                self.state.cursor_y = y;
                self.state.x = (x - anchor_x).max(0.0).min(viewport.width() - layout.width());
                self.state.y = (y - anchor_y).max(0.0).min(viewport.height() - layout.height());
            }

            (Event::Release(Key::LeftMouseButton), InnerState::Dragging(_, _)) => {
                self.state.inner = InnerState::Idle;
            }

            _ => (),
        }

        self.title.event(title, clip, event, context);
        self.content.event(content, clip, event, context);
    }

    fn draw(&mut self, viewport: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let (layout, title, content) = self.layout(viewport, style);

        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        result.extend(self.title.draw(title, clip));
        result.extend(self.content.draw(content, clip));
        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Window<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            inner: InnerState::Idle,
        }
    }
}

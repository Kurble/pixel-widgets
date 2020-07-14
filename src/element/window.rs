use crate::draw::*;
use crate::element::{Element, IntoNode, Node, Stylable};
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

pub struct Window<'a, T, S> {
    state: &'a mut State,
    content: Node<'a, T>,
    title: S,
    on_close: Option<T>,
}

pub struct State {
    pub x: f32,
    pub y: f32,
    cursor_x: f32,
    cursor_y: f32,
    inner: InnerState,
}

#[derive(Clone, Copy)]
enum InnerState {
    Idle,
    Dragging(f32, f32),
}

impl<'a, T: 'a, S: 'a + AsRef<str>> Window<'a, T, S> {
    pub fn new(state: &'a mut State, content: impl IntoNode<'a, T>, title: S) -> Self {
        Self {
            state,
            content: content.into_node(),
            title,
            on_close: None,
        }
    }

    pub fn closable(state: &'a mut State, content: impl IntoNode<'a, T>, title: S, on_close: T) -> Self {
        Self {
            state,
            content: content.into_node(),
            title,
            on_close: Some(on_close),
        }
    }

    fn layout(&self, viewport: Rectangle, style: &Stylesheet) -> (Rectangle, Rectangle) {
        let content_size = self.content.size();
        let width = content_size.0.min_size();
        let height = content_size.1.min_size();
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
            height + padding.top + padding.bottom
        );
        let content = layout.after_padding(padding);
        (layout, content)
    }
}

impl<'a, T: 'a, S: 'a + AsRef<str>> Element<'a, T> for Window<'a, T, S> {
    fn element(&self) -> &'static str {
        "window"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn Stylable<'a>)) {
        visitor(&mut self.content);
    }

    fn size(&self, _: &Stylesheet) -> (Size, Size) {
        (Size::Fill(1), Size::Fill(1))
    }

    fn event(&mut self, viewport: Rectangle, style: &Stylesheet, event: Event, clip: Rectangle) -> Option<T> {
        let (layout, content) = self.layout(viewport, style);
        
        match (event, self.state.inner) {
            (Event::Cursor(x, y), InnerState::Idle) => {
                self.state.cursor_x = x;
                self.state.cursor_y = y;
            }

            (Event::Press(Key::LeftMouseButton), InnerState::Idle) => {
                if clip.point_inside(self.state.cursor_x, self.state.cursor_y)
                    && !content.point_inside(self.state.cursor_x, self.state.cursor_y)
                    && layout.point_inside(self.state.cursor_x, self.state.cursor_y)
                {
                    self.state.inner = InnerState::Dragging(
                        self.state.cursor_x - layout.left,
                        self.state.cursor_y - layout.top
                    );
                }
            }

            (Event::Cursor(x, y), InnerState::Dragging(anchor_x, anchor_y)) => {
                self.state.cursor_x = x;
                self.state.cursor_y = y;
                self.state.x = (x - anchor_x).max(0.0).min(viewport.width() - layout.width());
                self.state.y = (y - anchor_y).max(0.0).min(viewport.height() - layout.height());
            }

            (Event::Release(Key::LeftMouseButton), InnerState::Dragging(_, _)) => {
                self.state.inner = InnerState::Idle;
            }
            
            _ => ()
        }

        self.content.event(content, event, clip)
    }

    fn render(&mut self, layout: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let (layout, content) = self.layout(layout, style);
        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        result.extend(self.content.render(content));
        result
    }
}

impl<'a, T: 'a, S: 'a + AsRef<str>> IntoNode<'a, T> for Window<'a, T, S> { }

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
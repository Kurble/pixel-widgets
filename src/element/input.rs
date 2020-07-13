use crate::draw::*;
use crate::element::{Element, IntoNode, Node};
use crate::event::{Event, Key, Modifiers};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::text::{Text, TextWrap};
use clipboard::{ClipboardContext, ClipboardProvider};
use rusttype::Scale;
use std::borrow::Cow;
use std::time::Instant;

pub struct State {
    scroll_x: f32,
    scroll_y: f32,
    modifiers: Modifiers,
    inner: InnerState,
    cursor: (f32, f32),
    buffer: String,
}

#[derive(Clone, Copy)]
enum InnerState {
    Dragging(usize, usize, Instant),
    Focused(usize, usize, Instant),
    Idle,
}

pub struct Input<'a, T, F> {
    placeholder: &'a str,
    state: &'a mut State,
    password: bool,
    on_change: F,
    on_submit: Option<T>,
}

impl<'a, T, F: Fn(String) -> T> Input<'a, T, F> {
    pub fn new(state: &'a mut State, placeholder: &'a str, on_change: F) -> Self {
        Input {
            placeholder,
            state,
            password: false,
            on_change,
            on_submit: None,
        }
    }

    pub fn password(state: &'a mut State, placeholder: &'a str, on_change: F) -> Self {
        Input {
            placeholder,
            state,
            password: true,
            on_change,
            on_submit: None,
        }
    }

    fn text(&self, stylesheet: &Stylesheet) -> Text {
        Text {
            text: Cow::Borrowed(&self.state.buffer),
            font: stylesheet.font.clone(),
            size: stylesheet.text_size,
            wrap: TextWrap::NoWrap,
            color: stylesheet.color,
        }
    }

    fn placeholder(&self, stylesheet: &Stylesheet) -> Text {
        Text {
            text: Cow::Borrowed(self.placeholder),
            font: stylesheet.font.clone(),
            size: stylesheet.text_size,
            wrap: TextWrap::NoWrap,
            color: stylesheet.color.with_alpha(0.5),
        }
    }

    fn content_rect(&self, layout: Rectangle, stylesheet: &Stylesheet) -> Rectangle {
        layout.after_padding(stylesheet.padding)
    }
}

impl<'a, T: 'a, F: 'a + Fn(String) -> T> Element<'a, T> for Input<'a, T, F> {
    fn element(&self) -> &'static str {
        "input"
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, T>)) {}

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        match (stylesheet.width, stylesheet.height) {
            (Size::Shrink, Size::Shrink) => {
                let width = self.placeholder(stylesheet).measure(None).width()
                    + stylesheet.padding.left
                    + stylesheet.padding.right;
                let metrics = stylesheet.font.inner.v_metrics(Scale::uniform(stylesheet.text_size));
                let height = metrics.ascent - metrics.descent + stylesheet.padding.top + stylesheet.padding.bottom;
                (Size::Exact(width), Size::Exact(height))
            }

            (Size::Shrink, other) => {
                let width = self.placeholder(stylesheet).measure(None).width()
                    + stylesheet.padding.left
                    + stylesheet.padding.right;
                (Size::Exact(width), other)
            }

            (other, Size::Shrink) => {
                let metrics = stylesheet.font.inner.v_metrics(Scale::uniform(stylesheet.text_size));
                let height = metrics.ascent - metrics.descent + stylesheet.padding.top + stylesheet.padding.bottom;
                (other, Size::Exact(height))
            }

            other => other,
        }
    }

    fn event(&mut self, layout: Rectangle, stylesheet: &Stylesheet, event: Event) -> Option<T> {
        let content_rect = self.content_rect(layout, stylesheet);
        let mut result = None;

        // sanity check on the state
        self.state.inner = match self.state.inner {
            InnerState::Dragging(mut from, mut to, since) => {
                if from > self.state.buffer.len() {
                    from = self.state.buffer.len();
                }
                if to > self.state.buffer.len() {
                    to = self.state.buffer.len();
                }
                InnerState::Dragging(from, to, since)
            }
            InnerState::Focused(mut from, mut to, since) => {
                if from > self.state.buffer.len() {
                    from = self.state.buffer.len();
                }
                if to > self.state.buffer.len() {
                    to = self.state.buffer.len();
                }
                InnerState::Focused(from, to, since)
            }
            InnerState::Idle => InnerState::Idle,
        };

        //if context.cursor.inside(&current) {
        //    context.style = MouseStyle::Text;
        //}

        // event related state update
        match event {
            Event::Cursor(x, y) => {
                self.state.cursor = (x, y);
                if let InnerState::Dragging(from, _, _) = self.state.inner {
                    let relative_cursor = (
                        self.state.cursor.0 - content_rect.left + self.state.scroll_x,
                        self.state.cursor.1 - content_rect.top + self.state.scroll_y,
                    );
                    let hit =
                        text_display(self.text(stylesheet), self.password).hitdetect(relative_cursor, content_rect);
                    self.state.inner = InnerState::Dragging(from, hit, Instant::now());
                }
            }

            Event::Modifiers(modifiers) => {
                self.state.modifiers = modifiers;
            }

            Event::Press(Key::LeftMouseButton) => {
                if layout.point_inside(self.state.cursor.0, self.state.cursor.1) {
                    let relative_cursor = (
                        self.state.cursor.0 - content_rect.left + self.state.scroll_x,
                        self.state.cursor.1 - content_rect.top + self.state.scroll_y,
                    );
                    let hit =
                        text_display(self.text(stylesheet), self.password).hitdetect(relative_cursor, content_rect);
                    self.state.inner = InnerState::Dragging(hit, hit, Instant::now());
                } else {
                    self.state.inner = InnerState::Idle;
                }
            }

            Event::Release(Key::LeftMouseButton) => {
                self.state.inner = match self.state.inner {
                    InnerState::Dragging(from, to, since) => InnerState::Focused(from, to, since),
                    other => other,
                }
            }

            event => {
                if let InnerState::Focused(from, to, since) = self.state.inner {
                    match event {
                        Event::Text(c) => match c {
                            '\x08' => {
                                let (from, to) = (from.min(to), from.max(to));

                                if to > from {
                                    let pt = codepoint(&self.state.buffer, from);
                                    let tail = self.state.buffer.split_off(pt);
                                    self.state.buffer.push_str(tail.split_at(codepoint(&tail, to - from)).1);
                                    self.state.inner = InnerState::Focused(from, from, Instant::now());
                                    result.replace((self.on_change)(self.state.buffer.clone()));
                                } else if from > 0 {
                                    let pt = codepoint(&self.state.buffer, from - 1);
                                    let tail = self.state.buffer.split_off(pt);
                                    self.state.buffer.push_str(tail.split_at(codepoint(&tail, 1)).1);
                                    self.state.inner = InnerState::Focused(from - 1, from - 1, Instant::now());
                                    result.replace((self.on_change)(self.state.buffer.clone()));
                                }
                            }
                            '\x7f' => {
                                let (from, to) = (from.min(to), from.max(to));

                                let pt = codepoint(&self.state.buffer, from);
                                let tail = self.state.buffer.split_off(pt);

                                if to > from {
                                    self.state.buffer.push_str(tail.split_at(codepoint(&tail, to - from)).1);
                                } else if tail.len() > 0 {
                                    self.state.buffer.push_str(tail.split_at(codepoint(&tail, 1)).1);
                                }
                                self.state.inner = InnerState::Focused(from, from, Instant::now());
                                result.replace((self.on_change)(self.state.buffer.clone()));
                            }
                            c => {
                                if !c.is_control() {
                                    let (from, to) = (from.min(to), from.max(to));

                                    let pt = codepoint(&self.state.buffer, from);
                                    let mut tail = self.state.buffer.split_off(pt);
                                    self.state.buffer.push(c);

                                    if to > from {
                                        let pt = codepoint(&tail, to - from);
                                        self.state.buffer.push_str(&tail.split_off(pt));
                                    } else {
                                        self.state.buffer.push_str(&tail);
                                    }
                                    self.state.inner = InnerState::Focused(from + 1, from + 1, Instant::now());
                                    result.replace((self.on_change)(self.state.buffer.clone()));
                                }
                            }
                        },

                        Event::Press(Key::Enter) => {
                            if self.state.modifiers.shift == false {
                                result = self.on_submit.take();
                            }
                        }

                        Event::Press(Key::C) => {
                            if self.state.modifiers.ctrl {
                                let (a, b) = (from.min(to), from.max(to));
                                let (a, b) = (codepoint(&self.state.buffer, a), codepoint(&self.state.buffer, b));
                                let copy_text = self.state.buffer[a..b].to_string();
                                ClipboardContext::new()
                                    .and_then(|mut cc| cc.set_contents(copy_text))
                                    .ok();
                            }
                        }

                        Event::Press(Key::X) => {
                            if self.state.modifiers.ctrl {
                                let (from, to) = (from.min(to), from.max(to));
                                let (a, b) = (codepoint(&self.state.buffer, from), codepoint(&self.state.buffer, to));
                                let cut_text = self.state.buffer[a..b].to_string();
                                ClipboardContext::new()
                                    .and_then(|mut cc| cc.set_contents(cut_text))
                                    .ok();

                                let pt = codepoint(&self.state.buffer, from);
                                let tail = self.state.buffer.split_off(pt);

                                if to > from {
                                    self.state.buffer.push_str(tail.split_at(codepoint(&tail, to - from)).1);
                                } else if tail.len() > 0 {
                                    self.state.buffer.push_str(tail.split_at(codepoint(&tail, 1)).1);
                                }
                                self.state.inner = InnerState::Focused(from, from, since);
                                result.replace((self.on_change)(self.state.buffer.clone()));
                            }
                        }

                        Event::Press(Key::V) => {
                            if self.state.modifiers.ctrl {
                                let (from, to) = (from.min(to), from.max(to));
                                let paste_text = ClipboardContext::new().and_then(|mut cc| cc.get_contents()).ok();

                                if let Some(paste_text) = paste_text {
                                    let pt = codepoint(&self.state.buffer, from);
                                    let mut tail = self.state.buffer.split_off(pt);
                                    self.state.buffer.push_str(&paste_text);

                                    if to > from {
                                        let pt = codepoint(&tail, to - from);
                                        self.state.buffer.push_str(&tail.split_off(pt));
                                    } else {
                                        self.state.buffer.push_str(&tail);
                                    }
                                    self.state.inner = InnerState::Focused(
                                        from + paste_text.len(),
                                        from + paste_text.len(),
                                        Instant::now(),
                                    );
                                    result.replace((self.on_change)(self.state.buffer.clone()));
                                }
                            }
                        }

                        Event::Press(Key::Left) => {
                            if self.state.modifiers.shift {
                                self.state.inner =
                                    InnerState::Focused(from, if to > 0 { to - 1 } else { 0 }, Instant::now());
                            } else {
                                let (from, to) = (from.min(to), from.max(to));
                                if from != to || from == 0 {
                                    self.state.inner = InnerState::Focused(from, from, Instant::now());
                                } else {
                                    self.state.inner = InnerState::Focused(from - 1, from - 1, Instant::now());
                                }
                            }
                        }

                        Event::Press(Key::Right) => {
                            if self.state.modifiers.shift {
                                let count = self.state.buffer.chars().count();
                                self.state.inner = InnerState::Focused(from, (to + 1).min(count), Instant::now());
                            } else {
                                let (from, to) = (from.min(to), from.max(to));
                                if from != to || to >= self.state.buffer.chars().count() {
                                    self.state.inner = InnerState::Focused(to, to, Instant::now());
                                } else {
                                    self.state.inner = InnerState::Focused(to + 1, to + 1, Instant::now());
                                }
                            }
                        }

                        Event::Press(Key::Home) => {
                            if self.state.modifiers.shift {
                                self.state.inner = InnerState::Focused(from, 0, Instant::now());
                            } else {
                                self.state.inner = InnerState::Focused(0, 0, Instant::now());
                            }
                        }

                        Event::Press(Key::End) => {
                            if self.state.modifiers.shift {
                                let count = self.state.buffer.chars().count();
                                self.state.inner = InnerState::Focused(from, count, Instant::now());
                            } else {
                                let count = self.state.buffer.chars().count();
                                self.state.inner = InnerState::Focused(count, count, Instant::now());
                            }
                        }

                        _ => (),
                    }
                }
            }
        }

        // update scroll state for current text and caret position
        match self.state.inner {
            InnerState::Dragging(_, pos, _) | InnerState::Focused(_, pos, _) => {
                let (caret, range) =
                    self.text(stylesheet)
                        .measure_range(pos, self.state.buffer.chars().count(), content_rect);

                if self.state.scroll_x + content_rect.width() > range.0 + 2.0 {
                    self.state.scroll_x = (range.0 - content_rect.width() + 2.0).max(0.0);
                }
                if caret.0 - self.state.scroll_x > content_rect.width() - 2.0 {
                    self.state.scroll_x = caret.0 - content_rect.width() + 2.0;
                }
                if caret.0 - self.state.scroll_x < 0.0 {
                    self.state.scroll_x = caret.0;
                }
                if caret.1 - self.state.scroll_y > content_rect.height() - 2.0 {
                    self.state.scroll_y = caret.1 - content_rect.height() + 2.0;
                }
                if caret.1 - self.state.scroll_y < 0.0 {
                    self.state.scroll_y = caret.1;
                }
            }
            _ => (),
        };

        result
    }

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();

        let content_rect = self.content_rect(layout, stylesheet);
        let text_rect = content_rect.translate(-self.state.scroll_x, -self.state.scroll_y);
        let text = text_display(self.text(stylesheet), self.password);

        result.extend(stylesheet.background.render(layout).into_iter());
        result.push(Primitive::PushClip(content_rect));
        match self.state.inner {
            InnerState::Dragging(from, to, since) | InnerState::Focused(from, to, since) => {
                let range = text.measure_range(from.min(to), from.max(to), text_rect);

                if to != from {
                    result.push(Primitive::DrawRect(
                        Rectangle {
                            left: text_rect.left + (range.0).0,
                            right: text_rect.left + (range.1).0,
                            top: text_rect.top,
                            bottom: text_rect.bottom,
                        },
                        Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.5,
                            a: 0.5,
                        },
                    ));
                }

                if since.elapsed().subsec_nanos() < 500_000_000 {
                    let caret = if to > from { range.1 } else { range.0 };

                    result.push(Primitive::DrawRect(
                        Rectangle {
                            left: text_rect.left + caret.0,
                            right: text_rect.left + caret.0 + 1.0,
                            top: text_rect.top,
                            bottom: text_rect.bottom,
                        },
                        Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        },
                    ));
                }
            }
            _ => (),
        }
        if self.state.buffer.is_empty() {
            result.push(Primitive::DrawText(self.placeholder(stylesheet).to_owned(), text_rect));
        } else {
            result.push(Primitive::DrawText(text, text_rect));
        }
        result.push(Primitive::PopClip);

        result
    }
}

impl<'a, T: 'a, F: 'a + Fn(String) -> T> IntoNode<'a, T> for Input<'a, T, F> {}

impl Default for State {
    fn default() -> Self {
        State {
            scroll_x: 0.0,
            scroll_y: 0.0,
            modifiers: Modifiers {
                ctrl: false,
                alt: false,
                shift: false,
                logo: false,
            },
            inner: InnerState::Idle,
            cursor: (0.0, 0.0),
            buffer: String::new(),
        }
    }
}

fn text_display(buffer: Text<'_>, password: bool) -> Text<'static> {
    if password {
        Text {
            text: Cow::Owned("\u{25cf}".repeat(buffer.text.chars().count())),
            font: buffer.font.clone(),
            size: buffer.size,
            color: buffer.color,
            wrap: buffer.wrap,
        }
    } else {
        buffer.to_owned()
    }
}

fn codepoint(s: &String, char_index: usize) -> usize {
    s.char_indices().skip(char_index).next().map_or(s.len(), |(i, _)| i)
}

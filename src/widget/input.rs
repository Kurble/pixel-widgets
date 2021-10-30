use std::borrow::Cow;
use std::time::Instant;

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};
use rusttype::Scale;

use crate::draw::*;
use crate::event::{Event, Key, Modifiers};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::text::{Text, TextWrap};
use crate::widget::{Context, Widget};

/// State for [`Input`](struct.Input.html)
pub struct State {
    scroll_x: f32,
    scroll_y: f32,
    modifiers: Modifiers,
    inner: InnerState,
    cursor: (f32, f32),
}

#[derive(Clone, Copy)]
enum InnerState {
    Dragging(usize, usize, Instant),
    Focused(usize, usize, Instant),
    Idle,
}

/// Editable text input
pub struct Input<'a, T, F, S> {
    placeholder: &'a str,
    password: bool,
    value: S,
    on_change: F,
    on_submit: Option<T>,
    trigger: Option<Key>,
}

impl<'a, T, F, S> Input<'a, T, F, S>
where
    T: 'a + Send,
    F: 'a + Send + Fn(String) -> T,
    S: 'a + Send + AsRef<str>,
{
    /// Construct a new `Input`
    pub fn new(placeholder: &'a str, value: S, on_change: F) -> Self {
        Input {
            placeholder,
            password: false,
            value,
            on_change,
            on_submit: None,
            trigger: None,
        }
    }

    /// Sets the placeholder text, which is displayed when the input has no value.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Construct a new `Input` that renders the text as dots, for passwords.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Sets the current text value of the input.
    pub fn val<N: AsRef<str>>(self, value: N) -> Input<'a, T, F, N> {
        Input {
            placeholder: self.placeholder,
            password: self.password,
            value,
            on_change: self.on_change,
            on_submit: self.on_submit,
            trigger: self.trigger,
        }
    }

    /// Sets the message to post when the text values changes.
    pub fn on_change<N: Fn(String) -> T>(self, on_change: N) -> Input<'a, T, N, S> {
        Input {
            placeholder: self.placeholder,
            password: self.password,
            value: self.value,
            on_change,
            on_submit: self.on_submit,
            trigger: self.trigger,
        }
    }

    /// Sets the message to post when the users submits using the enter key
    pub fn on_submit(mut self, message: T) -> Self {
        self.on_submit.replace(message);
        self
    }

    /// Sets a key that will trigger input focus
    pub fn trigger_key(mut self, key: Key) -> Self {
        self.trigger.replace(key);
        self
    }

    fn text(&self, stylesheet: &Stylesheet) -> Text {
        Text {
            text: Cow::Borrowed(self.value.as_ref()),
            font: stylesheet.font.clone(),
            size: stylesheet.text_size,
            wrap: TextWrap::NoWrap,
            color: stylesheet.color,
        }
    }

    fn placeholder_text(&self, stylesheet: &Stylesheet) -> Text {
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

impl<'a, T> Default for Input<'a, T, fn(String) -> T, &'static str> {
    fn default() -> Self {
        Self {
            placeholder: "",
            password: false,
            value: "",
            on_change: |_| panic!("on_change of `Input` must be set"),
            on_submit: None,
            trigger: None,
        }
    }
}

impl<'a, T, F, S> Widget<'a, T> for Input<'a, T, F, S>
where
    T: 'a + Send,
    F: 'a + Send + Fn(String) -> T,
    S: 'a + Send + AsRef<str>,
{
    type State = State;

    fn mount(&self) -> Self::State {
        State::default()
    }

    fn widget(&self) -> &'static str {
        "input"
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &State, stylesheet: &Stylesheet) -> (Size, Size) {
        match (stylesheet.width, stylesheet.height) {
            (Size::Shrink, Size::Shrink) => {
                let width = self.placeholder_text(stylesheet).measure(None).width()
                    + stylesheet.padding.left
                    + stylesheet.padding.right;
                let metrics = stylesheet.font.inner.v_metrics(Scale::uniform(stylesheet.text_size));
                let height = metrics.ascent - metrics.descent + stylesheet.padding.top + stylesheet.padding.bottom;
                (Size::Exact(width), Size::Exact(height))
            }

            (Size::Shrink, other) => {
                let width = self.placeholder_text(stylesheet).measure(None).width()
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

    fn event(
        &mut self,
        state: &mut State,
        layout: Rectangle,
        clip: Rectangle,
        stylesheet: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        let content_rect = self.content_rect(layout, stylesheet);
        let value_len = self.value.as_ref().chars().count();
        let mut new_text = None;

        // sanity check on the state
        state.inner = match state.inner {
            InnerState::Dragging(mut from, mut to, since) => {
                if from > value_len {
                    from = value_len;
                }
                if to > value_len {
                    to = value_len;
                }
                InnerState::Dragging(from, to, since)
            }
            InnerState::Focused(mut from, mut to, since) => {
                if from > value_len {
                    from = value_len;
                }
                if to > value_len {
                    to = value_len;
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
                state.cursor = (x, y);
                if let InnerState::Dragging(from, _, _) = state.inner {
                    let relative_cursor = (
                        state.cursor.0 - content_rect.left + state.scroll_x,
                        state.cursor.1 - content_rect.top + state.scroll_y,
                    );
                    let hit =
                        text_display(self.text(stylesheet), self.password).hitdetect(relative_cursor, content_rect);
                    state.inner = InnerState::Dragging(from, hit, Instant::now());
                    context.redraw();
                }
            }

            Event::Modifiers(modifiers) => {
                state.modifiers = modifiers;
            }

            Event::Press(Key::LeftMouseButton) => {
                context.redraw();
                if layout.point_inside(state.cursor.0, state.cursor.1)
                    && clip.point_inside(state.cursor.0, state.cursor.1)
                {
                    let relative_cursor = (
                        state.cursor.0 - content_rect.left + state.scroll_x,
                        state.cursor.1 - content_rect.top + state.scroll_y,
                    );
                    let hit =
                        text_display(self.text(stylesheet), self.password).hitdetect(relative_cursor, content_rect);
                    state.inner = InnerState::Dragging(hit, hit, Instant::now());
                } else {
                    state.inner = InnerState::Idle;
                }
            }

            Event::Release(Key::LeftMouseButton) => {
                state.inner = match state.inner {
                    InnerState::Dragging(from, to, since) => {
                        context.redraw();
                        InnerState::Focused(from, to, since)
                    }
                    other => other,
                }
            }

            event => match state.inner {
                InnerState::Idle => match event {
                    Event::Press(key) if Some(key) == self.trigger => {
                        state.inner = InnerState::Focused(0, self.value.as_ref().len(), Instant::now());
                        context.redraw();
                    }
                    _ => (),
                },

                InnerState::Focused(from, to, _) => match event {
                    Event::Text(c) => match c {
                        '\x08' => {
                            context.redraw();
                            let (from, to) = (from.min(to), from.max(to));

                            if to > from {
                                state.inner = InnerState::Focused(from, from, Instant::now());
                                let (head, tail) = self.value.as_ref().split_at(codepoint(self.value.as_ref(), from));
                                new_text.replace(format!("{}{}", head, tail.split_at(codepoint(tail, to - from)).1));
                            } else if from > 0 {
                                state.inner = InnerState::Focused(from - 1, from - 1, Instant::now());
                                let (head, tail) =
                                    self.value.as_ref().split_at(codepoint(self.value.as_ref(), from - 1));
                                new_text.replace(format!("{}{}", head, tail.split_at(codepoint(tail, 1)).1));
                            }
                        }
                        '\x7f' => {
                            context.redraw();
                            let (from, to) = (from.min(to), from.max(to));
                            state.inner = InnerState::Focused(from, from, Instant::now());

                            let (head, tail) = self.value.as_ref().split_at(codepoint(self.value.as_ref(), from));
                            if to > from {
                                new_text.replace(format!("{}{}", head, tail.split_at(codepoint(tail, to - from)).1));
                            } else if !tail.is_empty() {
                                new_text.replace(format!("{}{}", head, tail.split_at(codepoint(tail, 1)).1));
                            }
                        }
                        c => {
                            if !c.is_control() {
                                context.redraw();
                                let (from, to) = (from.min(to), from.max(to));
                                state.inner = InnerState::Focused(from + 1, from + 1, Instant::now());

                                let (head, tail) = self.value.as_ref().split_at(codepoint(self.value.as_ref(), from));
                                if to > from {
                                    new_text.replace(format!(
                                        "{}{}{}",
                                        head,
                                        c,
                                        tail.split_at(codepoint(tail, to - from)).1
                                    ));
                                } else {
                                    new_text.replace(format!("{}{}{}", head, c, tail));
                                }
                            }
                        }
                    },

                    Event::Press(Key::Enter) if self.on_submit.is_some() => {
                        if !state.modifiers.shift {
                            context.redraw();
                            context.extend(self.on_submit.take());
                            state.inner = InnerState::Idle;
                        }
                    }

                    #[cfg(feature = "clipboard")]
                    Event::Press(Key::C) => {
                        if state.modifiers.ctrl {
                            let (a, b) = (
                                codepoint(self.value.as_ref(), from.min(to)),
                                codepoint(self.value.as_ref(), from.max(to)),
                            );
                            let copy_text = self.value.as_ref()[a..b].to_string();
                            ClipboardContext::new()
                                .and_then(|mut cc| cc.set_contents(copy_text))
                                .ok();
                        }
                    }

                    #[cfg(feature = "clipboard")]
                    Event::Press(Key::X) => {
                        if state.modifiers.ctrl {
                            context.redraw();
                            let (from, to) = (from.min(to), from.max(to));
                            let (a, b) = (codepoint(self.value.as_ref(), from), codepoint(self.value.as_ref(), to));
                            let cut_text = self.value.as_ref()[a..b].to_string();
                            ClipboardContext::new()
                                .and_then(|mut cc| cc.set_contents(cut_text))
                                .ok();

                            state.inner = InnerState::Focused(from, from, Instant::now());
                            let (head, tail) = self.value.as_ref().split_at(codepoint(self.value.as_ref(), from));
                            if to > from {
                                new_text.replace(format!("{}{}", head, tail.split_at(codepoint(tail, to - from)).1));
                            } else if !tail.is_empty() {
                                new_text.replace(format!("{}{}", head, tail.split_at(codepoint(tail, 1)).1));
                            }
                        }
                    }

                    #[cfg(feature = "clipboard")]
                    Event::Press(Key::V) => {
                        if state.modifiers.ctrl {
                            context.redraw();
                            let (from, to) = (from.min(to), from.max(to));
                            let paste_text = ClipboardContext::new().and_then(|mut cc| cc.get_contents()).ok();

                            if let Some(paste_text) = paste_text {
                                let (head, tail) = self.value.as_ref().split_at(codepoint(self.value.as_ref(), from));
                                state.inner = InnerState::Focused(
                                    from + paste_text.len(),
                                    from + paste_text.len(),
                                    Instant::now(),
                                );
                                if to > from {
                                    new_text.replace(format!(
                                        "{}{}{}",
                                        head,
                                        paste_text,
                                        tail.split_at(codepoint(tail, to - from)).1
                                    ));
                                } else {
                                    new_text.replace(format!("{}{}{}", head, paste_text, tail));
                                }
                            }
                        }
                    }

                    Event::Press(Key::Left) => {
                        context.redraw();
                        if state.modifiers.shift {
                            state.inner = InnerState::Focused(from, if to > 0 { to - 1 } else { 0 }, Instant::now());
                        } else {
                            let (from, to) = (from.min(to), from.max(to));
                            if from != to || from == 0 {
                                state.inner = InnerState::Focused(from, from, Instant::now());
                            } else {
                                state.inner = InnerState::Focused(from - 1, from - 1, Instant::now());
                            }
                        }
                    }

                    Event::Press(Key::Right) => {
                        context.redraw();
                        if state.modifiers.shift {
                            state.inner = InnerState::Focused(from, (to + 1).min(value_len), Instant::now());
                        } else {
                            let (from, to) = (from.min(to), from.max(to));
                            if from != to || to >= value_len {
                                state.inner = InnerState::Focused(to, to, Instant::now());
                            } else {
                                state.inner = InnerState::Focused(to + 1, to + 1, Instant::now());
                            }
                        }
                    }

                    Event::Press(Key::Home) => {
                        context.redraw();
                        if state.modifiers.shift {
                            state.inner = InnerState::Focused(from, 0, Instant::now());
                        } else {
                            state.inner = InnerState::Focused(0, 0, Instant::now());
                        }
                    }

                    Event::Press(Key::End) => {
                        context.redraw();
                        if state.modifiers.shift {
                            state.inner = InnerState::Focused(from, value_len, Instant::now());
                        } else {
                            state.inner = InnerState::Focused(value_len, value_len, Instant::now());
                        }
                    }

                    _ => (),
                },

                _ => (),
            },
        }

        // update scroll state for current text and caret position
        match state.inner {
            InnerState::Dragging(_, pos, _) | InnerState::Focused(_, pos, _) => {
                let mut measure_text = Text {
                    text: Cow::Borrowed(new_text.as_ref().map(String::as_str).unwrap_or(self.value.as_ref())),
                    font: stylesheet.font.clone(),
                    size: stylesheet.text_size,
                    wrap: TextWrap::NoWrap,
                    color: stylesheet.color,
                };

                let measure_text_len = measure_text.text.chars().count();

                if self.password {
                    measure_text.text = Cow::Owned("\u{25cf}".repeat(measure_text_len));
                }

                let (caret, range) = measure_text.measure_range(pos, measure_text_len, content_rect);

                if state.scroll_x + content_rect.width() > range.0 + 2.0 {
                    context.redraw();
                    state.scroll_x = (range.0 - content_rect.width() + 2.0).max(0.0);
                }
                if caret.0 - state.scroll_x > content_rect.width() - 2.0 {
                    context.redraw();
                    state.scroll_x = caret.0 - content_rect.width() + 2.0;
                }
                if caret.0 - state.scroll_x < 0.0 {
                    context.redraw();
                    state.scroll_x = caret.0;
                }
                if caret.1 - state.scroll_y > content_rect.height() - 2.0 {
                    context.redraw();
                    state.scroll_y = caret.1 - content_rect.height() + 2.0;
                }
                if caret.1 - state.scroll_y < 0.0 {
                    context.redraw();
                    state.scroll_y = caret.1;
                }
            }
            _ => (),
        };

        if let Some(new_text) = new_text {
            context.push((self.on_change)(new_text));
        }
    }

    fn draw(
        &mut self,
        state: &mut State,
        layout: Rectangle,
        clip: Rectangle,
        stylesheet: &Stylesheet,
    ) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();

        let content_rect = self.content_rect(layout, stylesheet);
        let text_rect = content_rect.translate(-state.scroll_x, -state.scroll_y);
        let text = text_display(self.text(stylesheet), self.password);

        result.extend(stylesheet.background.render(layout).into_iter());
        if let Some(clip) = content_rect.intersect(&clip) {
            result.push(Primitive::PushClip(clip));
            match state.inner {
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
            if self.value.as_ref().is_empty() {
                result.push(Primitive::DrawText(
                    self.placeholder_text(stylesheet).to_owned(),
                    text_rect,
                ));
            } else {
                result.push(Primitive::DrawText(text, text_rect));
            }
            result.push(Primitive::PopClip);
        }

        result
    }
}

impl<'a, T, F, S> IntoNode<'a, T> for Input<'a, T, F, S>
where
    T: 'a + Send,
    F: 'a + Send + Fn(String) -> T,
    S: 'a + Send + AsRef<str>,
{
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

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
        }
    }
}

impl State {
    /// Returns whether the input state is currently focused and accepting input
    pub fn is_focused(&self) -> bool {
        matches!(self.inner, InnerState::Focused(_, _, _))
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

fn codepoint(s: &str, char_index: usize) -> usize {
    s.char_indices().nth(char_index).map_or(s.len(), |(i, _)| i)
}

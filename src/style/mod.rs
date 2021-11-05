#![doc = include_str!("../../style.md")]
use std::collections::HashMap;
use std::iter::Peekable;

use crate::bitset::BitSet;
use crate::cache::Cache;
use crate::draw::{Background, Color, ImageData, Patch};
use crate::layout::{Align, Direction, Rectangle, Size};
use crate::text::{Font, TextWrap};

/// Style building tools
pub mod builder;
mod parse;
mod tokenize;
pub(crate) mod tree;

use crate::graphics::Graphics;
use futures::future::Map;
use futures::FutureExt;
use parse::*;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokenize::*;

use builder::*;

/// Errors that can be encountered while loading a stylesheet
#[derive(Debug)]
pub enum Error {
    /// Syntax error
    Syntax(String, TokenPos),
    /// Unexpected end of file error
    Eof,
    /// Image loading error
    Image(image::ImageError),
    /// File input/output error
    Io(Box<dyn std::error::Error + Send + Sync>),
}

/// Container for all styling data.
pub struct Style {
    cache: Arc<Mutex<Cache>>,
    resolved: Mutex<HashMap<BitSet, Arc<Stylesheet>>>,
    default: Stylesheet,
    rule_tree: tree::RuleTree,
}

#[doc(hidden)]
pub trait ReadFn: 'static + Clone {
    type Future: Future<Output = anyhow::Result<Vec<u8>>>;

    fn read(&self, path: &Path) -> Self::Future;
}

impl<T, F, E> ReadFn for T
where
    T: 'static + Fn(&Path) -> F + Clone,
    F: Future<Output = Result<Vec<u8>, E>>,
    E: Into<anyhow::Error>,
{
    #[allow(clippy::type_complexity)]
    type Future = Map<F, fn(Result<Vec<u8>, E>) -> anyhow::Result<Vec<u8>>>;

    fn read(&self, path: &Path) -> Self::Future {
        (*self)(path).map(|r| r.map_err(|e| e.into()))
    }
}

/// A fully resolved stylesheet, passed by reference to [`Widget::draw`](../widget/trait.Widget.html).
/// Contains the resolved values of all possible style properties.
#[derive(Clone, Debug)]
pub struct Stylesheet {
    /// Widget width
    pub width: Size,
    /// Widget height
    pub height: Size,
    /// Background for the widget that full covers the layout rect
    pub background: Background,
    /// Amount of padding to use on each side of the content
    pub padding: Rectangle,
    /// Size of the margin on each side of the widget
    pub margin: Rectangle,
    /// Color to use for foreground drawing, including text
    pub color: Color,
    /// Font to use for text rendering
    pub font: Font,
    /// Size of text
    pub text_size: f32,
    /// Wrapping strategy for text
    pub text_wrap: TextWrap,
    /// Layout direction for widgets that support it (atm not text unfortunately..)
    pub direction: Direction,
    /// How to align children horizontally
    pub align_horizontal: Align,
    /// How to align children vertically
    pub align_vertical: Align,
    /// Flags
    pub flags: Vec<String>,
}

/// A style property and it's value
#[derive(Debug)]
pub enum Declaration<I = ImageId, P = PatchId, F = FontId> {
    /// no background
    BackgroundNone,
    /// background color
    BackgroundColor(Color),
    /// background image
    BackgroundImage(I, Color),
    /// background patch
    BackgroundPatch(P, Color),
    /// font
    Font(F),
    /// color
    Color(Color),
    /// padding
    Padding(Rectangle),
    /// padding left
    PaddingLeft(f32),
    /// Padding right
    PaddingRight(f32),
    /// Padding top
    PaddingTop(f32),
    /// Padding bottom
    PaddingBottom(f32),
    /// margin
    Margin(Rectangle),
    /// padding left
    MarginLeft(f32),
    /// Padding right
    MarginRight(f32),
    /// Padding top
    MarginTop(f32),
    /// Padding bottom
    MarginBottom(f32),
    /// text-size
    TextSize(f32),
    /// text-wrap
    TextWrap(TextWrap),
    /// width
    Width(Size),
    /// height
    Height(Size),
    /// layout-direction
    LayoutDirection(Direction),
    /// align-horizontal
    AlignHorizontal(Align),
    /// align-vertical
    AlignVertical(Align),
    /// flag: true;
    AddFlag(String),
    /// flag: false;
    RemoveFlag(String),
}

/// A selector that selects widgets that match some property.
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// Should be ignored when building
    Root,
    /// Match a widget
    Widget(SelectorWidget),
    /// Match a widget that is a direct child of the parent
    WidgetDirectChild(SelectorWidget),
    /// Match a widget that follows directly after the previous widget
    WidgetDirectAfter(SelectorWidget),
    /// Match a widget that follows after a previous widget
    WidgetAfter(SelectorWidget),
    /// Match the nth child widget modulo a number
    NthMod(usize, usize),
    /// Match the nth child widget counted from the last child widget modulo a number
    NthLastMod(usize, usize),
    /// Match the nth child widget
    Nth(usize),
    /// Match the nth child widget counted from the last child widget
    NthLast(usize),
    /// Match widgets that are the only child of their parent
    OnlyChild,
    /// Match widgets that have a class
    Class(String),
    /// Match widgets that are in a state
    State(StyleState<String>),
    /// Invert the nested selector
    Not(Box<Selector>),
}

/// Widget name as used in a `Selector`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorWidget {
    /// Match any widget
    Any,
    /// Match specific widgets
    Some(String),
}

/// Widget states
// !!Note: do not forget to add new variants to the eq impl!!
#[derive(Debug, Clone)]
pub enum StyleState<S: AsRef<str>> {
    /// When the mouse is over the widget
    Hover,
    /// When the mouse is clicking on the widget
    Pressed,
    /// When the widget is in a checked state (checkbox, radio button)
    Checked,
    /// When a widget is disabled
    Disabled,
    /// When a widget has input focus
    Focused,
    /// When a widget in an expanded state
    Open,
    /// When a widget is in a collapsed state
    Closed,
    /// When a drag widget is being dragged
    Drag,
    /// When a drop widget accepts a dragged widget before it's dropped
    Drop,
    /// When a drop widget denies a dragged widget
    DropDenied,
    /// Custom state for custom widgets
    Custom(S),
}

impl Style {
    /// Returns a new `StyleBuilder`.
    pub fn builder() -> StyleBuilder {
        StyleBuilder::default()
    }

    pub(crate) fn get(&self, style: &BitSet) -> Arc<Stylesheet> {
        let mut resolved = self.resolved.lock().unwrap();
        if let Some(existing) = resolved.get(style) {
            return existing.clone();
        }
        let mut computed = self.default.clone();
        for rule in self.rule_tree.iter_declarations(style) {
            rule.apply(&mut computed);
        }
        let result = Arc::new(computed);
        resolved.insert(style.clone(), result.clone());
        result
    }

    pub(crate) fn rule_tree(&self) -> &tree::RuleTree {
        &self.rule_tree
    }

    pub(crate) fn cache(&self) -> Arc<Mutex<Cache>> {
        self.cache.clone()
    }

    /// Retrieve a `Graphics` loader that can be used to load images
    pub fn graphics(&self) -> Graphics {
        Graphics { cache: self.cache() }
    }
}

impl std::fmt::Debug for Style {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Style").field("rule_tree", &self.rule_tree).finish()
    }
}

impl Stylesheet {
    /// Returns whether a flag is set in this stylesheet
    pub fn contains(&self, flag: &str) -> bool {
        self.flags.binary_search_by_key(&flag, |s| s.as_str()).is_ok()
    }
}

impl Declaration<ImageData, Patch, Font> {
    /// Apply values to a `Stylesheet`.
    pub fn apply(&self, stylesheet: &mut Stylesheet) {
        match self {
            Declaration::BackgroundNone => stylesheet.background = Background::None,
            Declaration::BackgroundColor(x) => stylesheet.background = Background::Color(*x),
            Declaration::BackgroundImage(x, y) => stylesheet.background = Background::Image(x.clone(), *y),
            Declaration::BackgroundPatch(x, y) => stylesheet.background = Background::Patch(x.clone(), *y),
            Declaration::Font(x) => stylesheet.font = x.clone(),
            Declaration::Color(x) => stylesheet.color = *x,
            Declaration::Padding(x) => stylesheet.padding = *x,
            Declaration::PaddingLeft(x) => stylesheet.padding.left = *x,
            Declaration::PaddingRight(x) => stylesheet.padding.right = *x,
            Declaration::PaddingTop(x) => stylesheet.padding.top = *x,
            Declaration::PaddingBottom(x) => stylesheet.padding.bottom = *x,
            Declaration::Margin(x) => stylesheet.margin = *x,
            Declaration::MarginLeft(x) => stylesheet.margin.left = *x,
            Declaration::MarginRight(x) => stylesheet.margin.right = *x,
            Declaration::MarginTop(x) => stylesheet.margin.top = *x,
            Declaration::MarginBottom(x) => stylesheet.margin.bottom = *x,
            Declaration::TextSize(x) => stylesheet.text_size = *x,
            Declaration::TextWrap(x) => stylesheet.text_wrap = *x,
            Declaration::Width(x) => stylesheet.width = *x,
            Declaration::Height(x) => stylesheet.height = *x,
            Declaration::LayoutDirection(x) => stylesheet.direction = *x,
            Declaration::AlignHorizontal(x) => stylesheet.align_horizontal = *x,
            Declaration::AlignVertical(x) => stylesheet.align_vertical = *x,
            Declaration::AddFlag(x) => {
                if let Err(insert_at) = stylesheet.flags.binary_search(x) {
                    stylesheet.flags.insert(insert_at, x.clone());
                }
            }
            Declaration::RemoveFlag(x) => {
                if let Ok(exists) = stylesheet.flags.binary_search(x) {
                    stylesheet.flags.remove(exists);
                }
            }
        }
    }
}

impl Selector {
    /// Match a sibling widget of the current rule. If this selector is not a sibling selector `None` is returned.
    pub fn match_sibling(&self, direct: bool, widget: &str) -> Option<bool> {
        match self {
            Selector::WidgetDirectAfter(ref sel_widget) => Some(direct && sel_widget.matches(widget)),
            Selector::WidgetAfter(ref sel_widget) => Some(sel_widget.matches(widget)),
            Selector::Not(ref selector) => selector.match_sibling(direct, widget).map(|b| !b),
            _ => None,
        }
    }

    /// Match a child widget of the current rule. If this selector is not a child selector `None` is returned.
    pub fn match_child(&self, direct: bool, widget: &str) -> Option<bool> {
        match self {
            Selector::Widget(ref sel_widget) => Some(sel_widget.matches(widget)),
            Selector::WidgetDirectChild(ref sel_widget) => Some(direct && sel_widget.matches(widget)),
            Selector::Not(ref selector) => selector.match_child(direct, widget).map(|b| !b),
            _ => None,
        }
    }

    /// Match parameters of the widget matched by the current rule.
    /// If this selector is not a meta selector `None` is returned.
    pub fn match_meta<S: AsRef<str>>(
        &self,
        state: &[StyleState<S>],
        class: &str,
        n: usize,
        len: usize,
    ) -> Option<bool> {
        match self {
            Selector::State(ref sel_state) => Some(state.iter().any(|state| state.eq(sel_state))),
            Selector::Class(ref sel_class) => Some(sel_class == class),
            Selector::Nth(num) => Some(n == *num),
            Selector::NthMod(num, den) => Some((n % *den) == *num),
            Selector::NthLast(num) => Some(len - 1 - n == *num),
            Selector::NthLastMod(num, den) => Some(((len - 1 - n) % *den) == *num),
            Selector::OnlyChild => Some(n == 0 && len == 1),
            Selector::Not(ref selector) => selector.match_meta(state, class, n, len).map(|b| !b),
            _ => None,
        }
    }
}

impl SelectorWidget {
    fn matches(&self, widget: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Some(ref select) => select == widget,
        }
    }
}

impl<A: AsRef<str>, B: AsRef<str>> PartialEq<StyleState<B>> for StyleState<A> {
    fn eq(&self, other: &StyleState<B>) -> bool {
        match (self, other) {
            (StyleState::Hover, StyleState::Hover) => true,
            (StyleState::Pressed, StyleState::Pressed) => true,
            (StyleState::Checked, StyleState::Checked) => true,
            (StyleState::Disabled, StyleState::Disabled) => true,
            (StyleState::Focused, StyleState::Focused) => true,
            (StyleState::Open, StyleState::Open) => true,
            (StyleState::Closed, StyleState::Closed) => true,
            (StyleState::Drag, StyleState::Drag) => true,
            (StyleState::Drop, StyleState::Drop) => true,
            (StyleState::DropDenied, StyleState::DropDenied) => true,
            (StyleState::Custom(a), StyleState::Custom(b)) => a.as_ref().eq(b.as_ref()),

            _ => false,
        }
    }
}

impl From<image::ImageError> for Error {
    fn from(error: image::ImageError) -> Self {
        Error::Image(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Syntax(error, pos) => write!(f, "Syntax error: {} at line {}:{}", error, pos.line, pos.col_start),
            Error::Eof => write!(f, "Unexpected end of file reached"),
            Error::Image(error) => write!(f, "Image decode error: {}", error),
            Error::Io(error) => write!(f, "I/O error: {}", error),
        }
    }
}

impl std::error::Error for Error {}

impl<'a> From<&'a str> for SelectorWidget {
    fn from(s: &'a str) -> Self {
        if s == "*" {
            SelectorWidget::Any
        } else {
            SelectorWidget::Some(s.into())
        }
    }
}

impl<'a> From<&'a str> for StyleState<String> {
    fn from(s: &'a str) -> Self {
        match s {
            "hover" => StyleState::Hover,
            "pressed" => StyleState::Pressed,
            "checked" => StyleState::Checked,
            "disabled" => StyleState::Disabled,
            "open" => StyleState::Open,
            "closed" => StyleState::Closed,
            "drag" => StyleState::Drag,
            "drop" => StyleState::Drop,
            "drop-denied" => StyleState::DropDenied,
            other => StyleState::Custom(other.into()),
        }
    }
}

//!
//! Style in pixel-widgets is defined using stylesheets. These stylesheets are loaded from a file, with a format that is a
//! syntactically a subset of css. The stylesheets are called `pwss` - *p*ixel-*w*idgets *s*tyle*s*heets.
//! # Features
//! - Select widgets, .classes and :states
//! - Select child widgets, sibling widgets
//!
//! # Example
//! ```ignore
//! column {
//!     align-horizontal: center;
//! }
//!
//! button {
//!     background: #444;
//!     padding: 5;
//! }
//!
//! button:hover {
//!     background: #666;
//! }
//!
//! button:pressed {
//!     background: #222;
//! }
//!
//! button:hover > text {
//!     color: #f00;
//! }
//!
//! text {
//!     text-size: 24;
//! }
//! ```
//!
//! The example sets a few properties on some of the widgets. Just try it out with the examples in the example
//! directory and see for yourself what the effect is.
//!
//! # Syntax
//! Each pwss file contains a collection of _rules_. Rules are a group of _declarations_ that are applied to _selected_
//! widgets.
//!
//! ## Rules
//! A selector has the following format:
//! ```ignore
//! <selector> <selector> ... {
//!     <property>: <value>;
//!     <property>: <value>;
//!     ...
//! }
//! ```
//! The first line expects some selectors. Class selectors can be differentiated
//! from widget selectors by adding a period in front, as in `.class`, and state selectors have a ':' in front.
//! ```ignore
//! window column button {
//!     background: @button.png;
//! }
//! ```
//! Entering multiple selectors like in this example will look for a `button` inside a `column` inside a `window`.
//!
//! ## Selectors
//! This table describes the supported selectors
//!
//! | selector | example | description |
//! |---|---|---|
//! | `*` | `*` | selects all widgets |
//! | `widget` | `text` | selects all text widgets |
//! | `.class` | `.fancy` | selects all widgets that have the class "fancy" |
//! | `.. widget` | `.fancy text` | selects all text widgets that are a descendant of a "fancy" classed widget |
//! | `>widget` | `.fancy > text` | selects all text widgets that are a direct child of a "fancy" classed widget |
//! | `+widget` | `.fancy + text` | selects all widgets that follow directly after a "fancy" classed widget |
//! | `~widget` | `.fancy ~ text` | selects all widgets that follow after a "fancy" classed widget |
//! | `:state` | `button:hover` | selects all buttons that are hovered by the mouse |
//! | `:nth-child(n)` | `text:nth-child(2)` | selects text widgets that are the third child of their parent |
//! | `:nth-last-child(n)` | `text:nth-last-child(2)` | selects text widgets that are the third child of their parent, counted from the last widget |
//! | `:nth-child(odd)` | `text:nth-child(odd)` | selects text widgets that are an odd child of their parent |
//! | `:nth-child(even)` | `text:nth-child(even)` | selects text widgets that are an even child of their parent |
//! | `:not(selector)` | `button:not(:pressed)` | selects button widgets that are not pressed |
//! | `:only-child` | `column > *:only-child` | selects the only child of a column when the column has only one child |
//!
//! ## Properties
//! The interior of a rule consists of a number of declarations. These declarations are what specifies style.
//! A declaration starts with a property, and each property has it's own associated format.
//! Take a look at the table to see what properties exist.
//!
//! | key | description | format |
//! |---|---|---|
//! | width | widget width | size |
//! | height | widget height | size |
//! | background | Background for the widget that full covers the layout rect | background |
//! | padding | Amount of padding to use on each side of the content | rectangle |
//! | padding-left | Amount of padding to use on the left side of the content | number |
//! | padding-right | Amount of padding to use on the right side of the content | number |
//! | padding-top | Amount of padding to use on the top side of the content | number |
//! | padding-bottom | Amount of padding to use on the bottom side of the content | number |
//! | margin | Amount of margin to use on each side of the widget | rectangle |
//! | margin-left | Amount of margin to use on the left side of the widget | number |
//! | margin-right | Amount of margin to use on the right side of the widget | number |
//! | margin-top | Amount of margin to use on the top side of the widget | number |
//! | margin-bottom | Amount of margin to use on the bottom side of the widget | number |
//! | font | Font to use for text rendering | url |
//! | color | Color to use for foreground drawing, including text | color |
//! | text_size | Size of text | number |
//! | text_wrap | Wrapping strategy for text | textwrap |
//! | layout-direction | Layout direction for widgets that support it | direction |
//! | align_horizontal | how to align children horizontally | align |
//! | align_vertical | how to align children vertically | align |
//!
//! ### Value syntax
//! | Type | Syntax | Notes |
//! |---|---|---|
//! | color | `#rgb`<br>`#rgba`<br>`#rrggbb`<br>`#rrggbbaa` | Examples:<br>`#fff`<br>`#ff00ff` |
//! | url | `"filename"` | An url in between quotes<br>`"image.png"`<br>`"font.ttf"` |
//! | number | floating point literal | A number, such as `2.0` or `42` |
//! | background | `<url>`<br>`<color>`<br>`image(<url>, <color>)`<br>`patch(<url>, <color>)`<br>`none` | If a url ends with `.9.png` it will be resolved as a 9 patch.<br>If your 9 slice doesn't end with `.9.png`, use `patch`. |
//! | rectangle | `<num>`<br>`<num> <num>`<br>`<num> <num> <num>`<br>`<num> <num> <num> <num>` | `all sides`<br>`top/bottom`, `right/left`<br>`top`, `right/left`, `bottom`<br>`top`, `right`, `bottom`, `left` |
//! | textwrap | `no-wrap`<br>`wrap`<br>`word-wrap` | |
//! | size | `<number>`<br>`fill(<number>)`<br>`exact(<number>)`<br>`shrink` | Just a number resolves to `exact` |
//! | direction | `top-to-bottom`<br>`left-to-right`<br>`right-to-left`<br>`bottom-to-top` | |
//! | align | `begin`<br>`center`<br>`end` | |
use std::collections::HashMap;
use std::iter::Peekable;

use crate::bitset::BitSet;
use crate::cache::Cache;
use crate::draw::{Background, Color, Image, Patch};
use crate::layout::{Align, Rectangle, Size, Direction};
use crate::text::{Font, TextWrap};
use crate::loader::Loader;

pub(crate) mod tree;
mod parse;
mod tokenize;

use parse::*;
use tokenize::*;
use std::sync::{Arc, Mutex};

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
    Io(Box<dyn std::error::Error + Send>),
}

/// A style loaded from a `.pwss` file.
pub struct Style {
    resolved: Mutex<HashMap<BitSet, Arc<Stylesheet>>>,
    default: Stylesheet,
    rule_tree: tree::RuleTree,
}

/// A fully resolved stylesheet, passed by reference to [`Widget::draw`](../widget/trait.Widget.html).
/// Contains the final versions of all possible rules.
#[derive(Clone)]
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

/// A property and a value
pub enum Declaration {
    /// background
    Background(Background),
    /// font
    Font(Font),
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

/// A stylesheet selector, which widgets have to match against.
#[derive(Clone, PartialEq)]
pub enum Selector {
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
#[derive(Clone, PartialEq, Eq)]
pub enum SelectorWidget {
    /// Match any widget
    Any,
    /// Match specific widgets
    Some(String),
}

/// Widget states
#[derive(Clone)]
pub enum StyleState<S: AsRef<str>> {
    /// When the mouse is over the widget
    Hover,
    /// When the mouse is clicking on the widget
    Pressed,
    /// When the widget is in a checked state (checkbox, radio button)
    Checked,
    /// When a widget is disabled
    Disabled,
    /// When a widget in an expanded state
    Open,
    /// When a widget is in a collapsed state
    Closed,
    /// Custom state for custom widgets
    Custom(S),
}

impl Style {
    /// Construct a new default style
    pub fn new(cache: Arc<Mutex<Cache>>) -> Self {
        Style {
            resolved: Mutex::new(HashMap::new()),
            rule_tree: tree::RuleTree::default(),
            default: Stylesheet {
                background: Background::None,
                font: cache.lock().unwrap().load_font(include_bytes!("default_font.ttf").to_vec()),
                color: Color::white(),
                padding: Rectangle::zero(),
                margin: Rectangle::zero(),
                text_size: 16.0,
                text_wrap: TextWrap::NoWrap,
                width: Size::Shrink,
                height: Size::Shrink,
                direction: Direction::LeftToRight,
                align_horizontal: Align::Begin,
                align_vertical: Align::Begin,
                flags: Vec::new(),
            },
        }
    }

    pub(crate) fn get(&self, style: &BitSet) -> Arc<Stylesheet> {
        let mut resolved = self.resolved.lock().unwrap();
        if let Some(existing) = resolved.get(style) {
            return existing.clone();
        }
        let mut computed = self.default.clone();
        for rule in self.rule_tree.iter_declarations(&style) {
            rule.apply(&mut computed);
        }
        let result = Arc::new(computed);
        resolved.insert(style.clone(), result.clone());
        result
    }

    pub(crate) fn rule_tree(&self) -> &tree::RuleTree {
        &self.rule_tree
    }

    /// Asynchronously load a stylesheet from a .pwss file. See the [module documentation](index.html) on how to write
    /// .pwss files.
    pub async fn load<L: Loader, U: AsRef<str>>(
        loader: Arc<L>,
        url: U,
        cache: Arc<Mutex<Cache>>,
    ) -> Result<Self, Error> {
        let text = String::from_utf8(loader.load(url).await.map_err(|e| Error::Io(Box::new(e)))?).unwrap();
        parse(tokenize(text)?, loader, cache).await
    }
}

impl Stylesheet {
    /// Returns whether a flag is set in this stylesheet
    pub fn contains(&self, flag: &str) -> bool {
        self.flags.binary_search_by_key(&flag, |s| s.as_str()).is_ok()
    }
}

impl Declaration {
    /// Apply values to a `Stylesheet`.
    pub fn apply(&self, stylesheet: &mut Stylesheet) {
        match self {
            &Declaration::Background(ref x) => stylesheet.background = x.clone(),
            &Declaration::Font(ref x) => stylesheet.font = x.clone(),
            &Declaration::Color(ref x) => stylesheet.color = x.clone(),
            &Declaration::Padding(ref x) => stylesheet.padding = x.clone(),
            &Declaration::PaddingLeft(x) => stylesheet.padding.left = x,
            &Declaration::PaddingRight(x) => stylesheet.padding.right = x,
            &Declaration::PaddingTop(x) => stylesheet.padding.top = x,
            &Declaration::PaddingBottom(x) => stylesheet.padding.bottom = x,
            &Declaration::Margin(ref x) => stylesheet.margin = x.clone(),
            &Declaration::MarginLeft(x) => stylesheet.margin.left = x,
            &Declaration::MarginRight(x) => stylesheet.margin.right = x,
            &Declaration::MarginTop(x) => stylesheet.margin.top = x,
            &Declaration::MarginBottom(x) => stylesheet.margin.bottom = x,
            &Declaration::TextSize(x) => stylesheet.text_size = x.clone(),
            &Declaration::TextWrap(ref x) => stylesheet.text_wrap = x.clone(),
            &Declaration::Width(ref x) => stylesheet.width = x.clone(),
            &Declaration::Height(ref x) => stylesheet.height = x.clone(),
            &Declaration::LayoutDirection(x) => stylesheet.direction = x,
            &Declaration::AlignHorizontal(ref x) => stylesheet.align_horizontal = x.clone(),
            &Declaration::AlignVertical(ref x) => stylesheet.align_vertical = x.clone(),
            &Declaration::AddFlag(ref x) => match stylesheet.flags.binary_search(x) {
                Err(insert_at) => {
                    stylesheet.flags.insert(insert_at, x.clone());
                },
                Ok(_) => (),
            }
            &Declaration::RemoveFlag(ref x) => match stylesheet.flags.binary_search(x) {
                Ok(exists) => {
                    stylesheet.flags.remove(exists);
                },
                Err(_) => (),
            }
        }
    }
}

impl Selector {
    /// Match a sibling widget of the current rule. If this selector is not a sibling selector `None` is returned.
    pub fn match_sibling(&self, direct: bool, widget: &str) -> Option<bool> {
        match self {
            &Selector::WidgetDirectAfter(ref sel_widget) => Some(direct && sel_widget.matches(widget)),
            &Selector::WidgetAfter(ref sel_widget) => Some(sel_widget.matches(widget)),
            &Selector::Not(ref selector) => selector.match_sibling(direct, widget).map(|b| !b),
            &_ => None,
        }
    }

    /// Match a child widget of the current rule. If this selector is not a child selector `None` is returned.
    pub fn match_child(&self, direct: bool, widget: &str) -> Option<bool> {
        match self {
            &Selector::Widget(ref sel_widget) => Some(sel_widget.matches(widget)),
            &Selector::WidgetDirectChild(ref sel_widget) => Some(direct && sel_widget.matches(widget)),
            &Selector::Not(ref selector) => selector.match_child(direct, widget).map(|b| !b),
            &_ => None,
        }
    }

    /// Match parameters of the widget matched by the current rule.
    /// If this selector is not a meta selector `None` is returned.
    pub fn match_meta<S: AsRef<str>>(&self, state: &[StyleState<S>], class: &str, n: usize, len: usize) -> Option<bool> {
        match self {
            &Selector::State(ref sel_state) => Some(state.iter().find(|&state| state.eq(sel_state)).is_some()),
            &Selector::Class(ref sel_class) => Some(sel_class == class),
            &Selector::Nth(num) => Some(n == num),
            &Selector::NthMod(num, den) => Some((n % den) == num),
            &Selector::NthLast(num) => Some(len - 1 - n == num),
            &Selector::NthLastMod(num, den) => Some(((len - 1 - n) % den) == num),
            &Selector::OnlyChild => Some(n == 0 && len == 1),
            &Selector::Not(ref selector) => selector.match_meta(state, class, n, len).map(|b| !b),
            &_ => None,
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
            (StyleState::Open, StyleState::Open) => true,
            (StyleState::Closed, StyleState::Closed) => true,
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

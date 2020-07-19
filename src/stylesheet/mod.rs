//!
//! Style in pixel-widgets is defined using stylesheets. These stylesheets are loaded from a file, with a format that is a
//! syntactically a subset of css. The stylesheets are called `pwss` - *p*ixel-*w*idgets *s*tyle*s*heets.
//! # Features
//! - Select widgets by widget type and descendant type
//! - Select widgets by class and descendant class
//! - Stack multiple selectors that select the same widget
//!
//! # Example
//! ```ignore
//! column {
//!     align-horizontal: center
//! }
//!
//! button {
//!     background: #444
//!     hover: #668
//!     padding: 5
//! }
//!
//! text {
//!     text-size: 24
//! }
//! ```
//!
//! The example sets a few of the keys on some of the widgets. Just try it out with the examples in the example
//! directory and see for yourself what the effect is.
//!
//! # Syntax
//! Each pwss file contains a collection of _selectors_. Selectors are a group of _rules_ that are applied to _selected_
//! widgets.
//!
//! ## Selectors
//! A selector has the following format:
//! ```ignore
//! <widget...> <class...> {
//!     <rule...>
//! }
//! ```
//! The first line expects some widget identifiers and some class identifiers. Class identifiers can differentiated
//! from widget identifiers by adding a period in front, as in `.class`.
//! ```ignore
//! window column button {
//!     background: @button.png
//! }
//! ```
//! Entering multiple widgets that the selector will look for a `button` inside a `column` inside a `window`.
//!
//! ## Rules
//! The interior of a selector consists of a number of rules. These rules are what specifies style.
//! A rule starts with a key, and each key has it's own associated format. Take a look at the table to see what keys
//! exist.
//!
//! | key | description | format |
//! |---|---|---|
//! | background | Background for the widget that full covers the layout rect | background |
//! | hover | Background for button like widgets that are hovered | background |
//! | pressed | Background for button like widgets that are pressed | background |
//! | disabled | Background for button like widgets that are disabled | background |
//! | checked | Background for toggle like widgets that are checked | background |
//! | font | Font to use for text rendering | url |
//! | color | Color to use for foreground drawing, including text | color |
//! | scrollbar_horizontal | Graphics to use for horizontal scrollbars | background |
//! | scrollbar_vertical | Graphics to use for vertical scrollbars | background |
//! | padding | Amount of padding to use on each side of the content | rectangle |
//! | text_size | Size of text | number |
//! | text_wrap | Wrapping strategy for text | textwrap |
//! | width | widget width | size |
//! | height | widget height | size |
//! | align_horizontal | how to align children horizontally | align |
//! | align_vertical | how to align children vertically | align |
//!
//! ### Value syntax
//! | Type | Syntax | Notes |
//! |---|---|---|
//! | color | `#rgb`<br>`#rgba`<br>`#rrggbb`<br>`#rrggbbaa` | Examples:<br>`#fff`<br>`#ff00ff` |
//! | url | `@filename` | An `@` followed by an url<br>`@image.png`<br>`@font.ttf` |
//! | number | floating point literal | A number, such as `2.0` or `42` |
//! | background | `<url>`<br>`<color>`<br>`image(<url>, <color>)`<br>`patch(<url>, <color>)`<br>`none` | If a url ends with `.9.png` it will be resolved as a 9 patch. If your 9 slice doesn't end with `.9.png`, use `patch`. |
//! | rectangle | `<number>`<br>`(left: <number> right: <number> top: <number> bottom: <number>)` | All fields are optional as they default to zero:<br>`(left: 2 right: 2)` |
//! | textwrap | `no-wrap`<br>`wrap`<br>`word-wrap` | |
//! | size | `<number>`<br>`fill(<number>)`<br>`exact(<number>)`<br>`shrink` | Just a number resolves to `exact` |
//! | align | `begin`<br>`center`<br>`end` | |


use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::rc::Rc;

use crate::cache::Cache;
use crate::draw::{Background, Color, Image, Patch};
use crate::layout::{Align, Rectangle, Size};
use crate::text::{Font, TextWrap};
use crate::Loader;
use std::iter::Peekable;

mod parse;
mod tokenize;

use parse::*;
use tokenize::*;

/// Errors that can be encountered while loading a stylesheet
#[derive(Debug)]
pub enum Error<E: std::error::Error> {
    /// Syntax error
    Syntax(String, TokenPos),
    /// Unexpected end of file error
    Eof,
    /// Image loading error
    Image(image::ImageError),
    /// File input/output error
    Io(E),
}

/// A style loaded from a `.pwss` file.
pub struct Style {
    resolved: HashMap<Query<'static>, Rc<Stylesheet>>,
    default: Rc<Stylesheet>,
    selectors: Vec<Selector>,
}

/// A fully resolved stylesheet, passed by reference to [`Widget::draw`](../widget/trait.Widget.html).
/// Contains the final versions of all possible rules.
#[derive(Clone)]
pub struct Stylesheet {
    /// Background for the widget that full covers the layout rect
    pub background: Background,
    /// Background for button like widgets that are hovered
    pub hover: Background,
    /// Background for button like widgets that are pressed
    pub pressed: Background,
    /// Background for button like widgets that are disabled
    pub disabled: Background,
    /// Background for toggle like widgets that are checked
    pub checked: Background,
    /// Font to use for text rendering
    pub font: Font,
    /// Color to use for foreground drawing, including text
    pub color: Color,
    /// Graphics to use for horizontal scrollbars
    pub scrollbar_horizontal: Background,
    /// Graphics to use for vertical scrollbars
    pub scrollbar_vertical: Background,
    /// Amount of padding to use on each side of the content
    pub padding: Rectangle,
    /// Size of text
    pub text_size: f32,
    /// Wrapping strategy for text
    pub text_wrap: TextWrap,
    /// Widget width
    pub width: Size,
    /// Widget height
    pub height: Size,
    /// How to align children horizontally
    pub align_horizontal: Align,
    /// How to align children vertically
    pub align_vertical: Align,
}

enum Rule {
    Background(Background),
    Hover(Background),
    Pressed(Background),
    Disabled(Background),
    Checked(Background),
    Font(Font),
    Color(Color),
    ScrollbarHorizontal(Background),
    ScrollbarVertical(Background),
    Padding(Rectangle),
    TextSize(f32),
    TextWrap(TextWrap),
    Width(Size),
    Height(Size),
    AlignHorizontal(Align),
    AlignVertical(Align),
}

struct Selector {
    widgets: Vec<String>,
    classes: Vec<String>,
    rules: Vec<Rule>,
}

#[derive(Default, Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct Query<'a> {
    pub widgets: Vec<&'static str>,
    pub classes: Vec<Cow<'a, str>>,
}

impl Selector {
    fn matches(&self, query: &Query) -> bool {
        if !self.classes.is_empty() && query.classes.last().map(Cow::borrow) != self.classes.last().map(String::as_str)
        {
            return false;
        }

        if !self.widgets.is_empty() && query.widgets.last().cloned() != self.widgets.last().map(String::as_str) {
            return false;
        }

        let mut q = query.widgets.iter();
        if !self
            .widgets
            .iter()
            .fold(true, |m, d| m && q.find(|&x| x == d).is_some())
        {
            return false;
        }

        let mut q = query.classes.iter();
        if !self
            .classes
            .iter()
            .fold(true, |m, d| m && q.find(|x| x.as_ref() == d).is_some())
        {
            return false;
        }

        return true;
    }
}

impl Style {
    /// Construct a new default style
    pub fn new(cache: &mut Cache) -> Self {
        Style {
            resolved: HashMap::new(),
            selectors: Vec::new(),
            default: Rc::new(Stylesheet {
                background: Background::None,
                hover: Background::None,
                pressed: Background::None,
                disabled: Background::None,
                checked: Background::None,
                font: cache.load_font(include_bytes!("../../default_font.ttf").to_vec()),
                color: Color::white(),
                scrollbar_horizontal: Background::Color(Color::white()),
                scrollbar_vertical: Background::Color(Color::white()),
                padding: Rectangle::zero(),
                text_size: 16.0,
                text_wrap: TextWrap::NoWrap,
                width: Size::Shrink,
                height: Size::Shrink,
                align_horizontal: Align::Begin,
                align_vertical: Align::Begin,
            }),
        }
    }

    pub(crate) fn get(&mut self, query: &Query) -> Rc<Stylesheet> {
        if let Some(sheet) = self.resolved.get(query) {
            sheet.clone()
        } else {
            let mut stylesheet = (*self.default).clone();
            for selector in self.selectors.iter().filter(|s| s.matches(query)) {
                for rule in selector.rules.iter() {
                    rule.apply(&mut stylesheet);
                }
            }
            let stylesheet = Rc::new(stylesheet);
            self.resolved.insert(query.to_static(), stylesheet.clone());
            stylesheet
        }
    }

    /// Asynchronously load a stylesheet from a .pwss file. See the [module documentation](index.html) on how to write
    /// .pwss files.
    pub async fn load<L: Loader, U: AsRef<str>>(
        loader: &L,
        url: U,
        cache: &mut Cache,
    ) -> Result<Self, Error<L::Error>> {
        let text = String::from_utf8(loader.load(url).await.map_err(Error::Io)?).unwrap();
        parse(tokenize(text)?, loader, cache).await
    }
}

impl Rule {
    pub fn apply(&self, stylesheet: &mut Stylesheet) {
        match self {
            Rule::Background(x) => stylesheet.background = x.clone(),
            Rule::Hover(x) => stylesheet.hover = x.clone(),
            Rule::Pressed(x) => stylesheet.pressed = x.clone(),
            Rule::Disabled(x) => stylesheet.disabled = x.clone(),
            Rule::Checked(x) => stylesheet.checked = x.clone(),
            Rule::Font(x) => stylesheet.font = x.clone(),
            Rule::Color(x) => stylesheet.color = x.clone(),
            Rule::ScrollbarHorizontal(x) => stylesheet.scrollbar_horizontal = x.clone(),
            Rule::ScrollbarVertical(x) => stylesheet.scrollbar_vertical = x.clone(),
            Rule::Padding(x) => stylesheet.padding = x.clone(),
            Rule::TextSize(x) => stylesheet.text_size = x.clone(),
            Rule::TextWrap(x) => stylesheet.text_wrap = x.clone(),
            Rule::Width(x) => stylesheet.width = x.clone(),
            Rule::Height(x) => stylesheet.height = x.clone(),
            Rule::AlignHorizontal(x) => stylesheet.align_horizontal = x.clone(),
            Rule::AlignVertical(x) => stylesheet.align_vertical = x.clone(),
        }
    }
}

impl<'a> Query<'a> {
    pub fn to_static(&self) -> Query<'static> {
        Query {
            widgets: self.widgets.clone(),
            classes: self
                .classes
                .iter()
                .map(|x| Cow::Owned(x.clone().into_owned()))
                .collect(),
        }
    }
}

impl<E: std::error::Error> From<image::ImageError> for Error<E> {
    fn from(error: image::ImageError) -> Self {
        Error::Image(error)
    }
}

impl<E: std::error::Error> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Syntax(error, pos) => write!(f, "Syntax error: {} at line {}:{}", error, pos.line, pos.col_start),
            Error::Eof => write!(f, "Unexpected end of file reached"),
            Error::Image(error) => write!(f, "Image decode error: {}", error),
            Error::Io(error) => write!(f, "I/O error: {}", error),
        }
    }
}

impl<E: std::error::Error> std::error::Error for Error<E> {}

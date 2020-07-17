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

#[derive(Debug)]
pub enum Error<E: std::error::Error> {
    Syntax(String, TokenPos),
    Eof,
    Image(image::ImageError),
    Io(E),
}

pub struct Style {
    resolved: HashMap<Query<'static>, Rc<Stylesheet>>,
    default: Rc<Stylesheet>,
    selectors: Vec<Selector>,
}

#[derive(Clone)]
pub struct Stylesheet {
    pub background: Background,
    pub hover: Background,
    pub pressed: Background,
    pub disabled: Background,
    pub checked: Background,
    pub font: Font,
    pub color: Color,
    pub scrollbar_horizontal: Background,
    pub scrollbar_vertical: Background,
    pub padding: Rectangle,
    pub text_size: f32,
    pub text_wrap: TextWrap,
    pub width: Size,
    pub height: Size,
    pub align_horizontal: Align,
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
    elements: Vec<String>,
    classes: Vec<String>,
    rules: Vec<Rule>,
}

#[derive(Default, Debug, PartialEq, Eq, Hash, Clone)]
pub struct Query<'a> {
    pub elements: Vec<&'static str>,
    pub classes: Vec<Cow<'a, str>>,
}

impl Selector {
    fn matches(&self, query: &Query) -> bool {
        if !self.classes.is_empty() && query.classes.last().map(Cow::borrow) != self.classes.last().map(String::as_str)
        {
            return false;
        }

        if !self.elements.is_empty() && query.elements.last().cloned() != self.elements.last().map(String::as_str) {
            return false;
        }

        let mut q = query.elements.iter();
        if !self
            .elements
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

    pub fn get(&mut self, query: &Query) -> Rc<Stylesheet> {
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
            elements: self.elements.clone(),
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

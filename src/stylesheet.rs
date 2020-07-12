use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::rc::Rc;

use crate::cache::Cache;
use crate::draw::{Background, Color, Image, Patch};
use crate::layout::{Align, Rectangle, Size};
use crate::text::{Font, TextWrap};
use crate::Loader;
use std::iter::Peekable;

#[derive(Debug)]
pub enum Error<E: std::error::Error> {
    Syntax(String),
    Image(image::ImageError),
    Custom(E),
    FontMissing,
    ColorParse,
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

#[derive(Debug, Clone, Copy)]
struct TokenPos {
    line: usize,
    col_start: usize,
    col_end: usize,
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Identifier(String),
    Class(String),
    String(String),
    Number(String),
    Color(String),
    BraceOpen,
    BraceClose,
    CurlyOpen,
    CurlyClose,
    Colon,
    Semi,
    Comma,
}

#[derive(Debug)]
struct PositionedToken(Token, TokenPos);

struct LoadContext<'a, I: Iterator<Item = PositionedToken>, L: Loader> {
    loader: &'a L,
    tokens: Peekable<I>,
    cache: &'a mut Cache,
    images: &'a mut HashMap<String, Image>,
    patches: &'a mut HashMap<String, Patch>,
    fonts: &'a mut HashMap<String, Font>,
}

impl Selector {
    pub fn matches(&self, query: &Query) -> bool {
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
                font: cache.load_font(include_bytes!("../default_font.ttf").to_vec()),
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
        let mut result = Style::new(&mut *cache);
        let mut pos = TokenPos {
            line: 1,
            col_start: 0,
            col_end: 0,
        };

        let text = String::from_utf8(loader.load(url).await.map_err(Error::Custom)?).unwrap();
        let mut current = None;
        let mut tokens: Vec<PositionedToken> = text
            .chars()
            .filter_map(|c| {
                pos.col_end += 1;
                let token = match c {
                    '\n' => {
                        pos.col_start = 1;
                        pos.col_end = 1;
                        pos.line += 1;
                        None
                    }
                    c if c.is_alphabetic() => match &mut current {
                        Some(Token::Identifier(ref mut s))
                        | Some(Token::Class(ref mut s))
                        | Some(Token::String(ref mut s))
                        | Some(Token::Color(ref mut s)) => {
                            s.push(c);
                            None
                        }
                        current => current.replace(Token::Identifier(c.to_string())),
                    },
                    c if c.is_numeric() => match &mut current {
                        Some(Token::Number(ref mut s))
                        | Some(Token::Identifier(ref mut s))
                        | Some(Token::Class(ref mut s))
                        | Some(Token::String(ref mut s))
                        | Some(Token::Color(ref mut s)) => {
                            s.push(c);
                            None
                        }
                        current => current.replace(Token::Number(c.to_string())),
                    },
                    '-' | '_' => match &mut current {
                        Some(Token::Identifier(ref mut s))
                        | Some(Token::Class(ref mut s))
                        | Some(Token::String(ref mut s)) => {
                            s.push(c);
                            None
                        }
                        current => current.replace(Token::Identifier(c.to_string())),
                    },
                    '.' => match &mut current {
                        Some(Token::String(ref mut s)) | Some(Token::Number(ref mut s)) => {
                            s.push(c);
                            None
                        }
                        current => current.replace(Token::Class(String::new())),
                    },
                    '#' => match &mut current {
                        Some(Token::String(ref mut s)) => {
                            s.push(c);
                            None
                        }
                        current => current.replace(Token::Color(String::new())),
                    },
                    '"' => match &mut current {
                        Some(Token::String(_)) => current.take(),
                        current => current.replace(Token::String(String::new())),
                    },
                    '(' => current.replace(Token::BraceOpen),
                    ')' => current.replace(Token::BraceClose),
                    '{' => current.replace(Token::CurlyOpen),
                    '}' => current.replace(Token::CurlyClose),
                    ':' => current.replace(Token::Colon),
                    ';' => current.replace(Token::Semi),
                    ',' => current.replace(Token::Comma),
                    c if c.is_whitespace() => current.take(),
                    _ => panic!(),
                };
                token.map(|token| {
                    let result = PositionedToken(token, pos);
                    pos.col_start = pos.col_end;
                    result
                })
            })
            .collect();
        tokens.extend(current.take().map(|tok| PositionedToken(tok, pos)));

        let mut images = HashMap::new();
        let mut patches = HashMap::new();
        let mut fonts = HashMap::new();
        let mut context = LoadContext {
            loader,
            tokens: tokens.into_iter().peekable(),
            cache,
            images: &mut images,
            patches: &mut patches,
            fonts: &mut fonts,
        };

        while let Some(_) = context.tokens.peek() {
            result.selectors.push(parse_selector(&mut context).await?);
        }

        Ok(result)
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

impl<I: Iterator<Item = PositionedToken>, L: Loader> LoadContext<'_, I, L> {
    pub fn take(&mut self, token: Token) -> Result<PositionedToken, Error<L::Error>> {
        let next = self.tokens.next().ok_or(Error::Syntax(format!("Expected '{:?}' at end of file", token)))?;
        if token == next.0 {
            Ok(next)
        } else {
            Err(Error::Syntax(format!("Expected '{:?}' at {}:{}, found '{:?}'", token, next.1.line, next.1.col_start, next.0)))
        }
    }
}

async fn parse_selector<I: Iterator<Item = PositionedToken>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Selector, Error<L::Error>> {
    let mut selector = Selector {
        elements: vec![],
        classes: vec![],
        rules: vec![],
    };

    loop {
        match c.tokens.next() {
            Some(PositionedToken(Token::Identifier(element), _)) => {
                selector.elements.push(element);
            }
            Some(PositionedToken(Token::Class(class), _)) => {
                selector.classes.push(class);
            }
            Some(PositionedToken(Token::CurlyOpen, _)) => {
                loop {
                    if let Some(&PositionedToken(Token::CurlyClose, _)) = c.tokens.peek() {
                        break;
                    } else {
                        selector.rules.push(parse_rule(c).await?);
                    }
                }
                c.take(Token::CurlyClose)?;
                return Ok(selector);
            }
            Some(other) => return Err(Error::Syntax(format!("Unexpected token parsing selector: {:?}", other))),
            None => return Err(Error::Syntax("unexpected end of file".into())),
        }
    }
}

async fn parse_rule<I: Iterator<Item = PositionedToken>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Rule, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::Identifier(key), _)) => {
            c.take(Token::Colon)?;
            println!("{}", key);
            match key.as_str() {
                "background" => Ok(Rule::Background(parse_background(c).await?)),
                "hover" => Ok(Rule::Hover(parse_background(c).await?)),
                "pressed" => Ok(Rule::Pressed(parse_background(c).await?)),
                "disabled" => Ok(Rule::Disabled(parse_background(c).await?)),
                "checked" => Ok(Rule::Checked(parse_background(c).await?)),
                "font" => Ok(Rule::Font(parse_font(c).await?)),
                "color" => Ok(Rule::Color(parse_color::<L>(c.tokens.next())?)),
                "scrollbar-horizontal" => Ok(Rule::ScrollbarHorizontal(parse_background(c).await?)),
                "scrollbar-vertical" => Ok(Rule::ScrollbarVertical(parse_background(c).await?)),
                "padding" => Ok(Rule::Padding(parse_rectangle(c)?)),
                "text-size" => Ok(Rule::TextSize(parse_float(c)?)),
                "text-wrap" => Ok(Rule::TextWrap(parse_text_wrap(c)?)),
                "width" => Ok(Rule::Width(parse_size(c)?)),
                "height" => Ok(Rule::Height(parse_size(c)?)),
                "align-horizontal" => Ok(Rule::AlignHorizontal(parse_align(c)?)),
                "align-vertical" => Ok(Rule::AlignVertical(parse_align(c)?)),
                unrecognized => Err(Error::Syntax(format!("Key {} not recognized", unrecognized))),
            }
        }
        Some(PositionedToken(tok, pos)) => {
            Err(Error::Syntax(format!("{}:{}: Expected identifier at start of rule, got '{:?}'", pos.line, pos.col_start, tok)))
        }
        None => {
            Err(Error::Syntax("Unexpected end of file parsing rule".into()))
        }
    }
}

async fn parse_background<I: Iterator<Item = PositionedToken>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Background, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "none" => Ok(Background::None),
            "image" => {
                c.take(Token::BraceOpen)?;
                let image = if let Some(PositionedToken(Token::String(url), _)) = c.tokens.next() {
                    if c.images.get(&url).is_none() {
                        let image =
                            image::load_from_memory(c.loader.load(url.clone()).await.map_err(Error::Custom)?.as_ref())?;
                        c.images.insert(url.clone(), c.cache.load_image(image.to_rgba()));
                    }
                    Ok(c.images[&url].clone())
                } else {
                    Err(Error::Syntax("Expected url".into()))
                }?;
                c.take(Token::Comma)?;
                let color = parse_color::<L>(c.tokens.next())?;
                c.take(Token::BraceClose)?;
                Ok(Background::Image(image, color))
            }
            "patch" => {
                c.take(Token::BraceOpen)?;
                let image = if let Some(PositionedToken(Token::String(url), _)) = c.tokens.next() {
                    if c.patches.get(&url).is_none() {
                        let image =
                            image::load_from_memory(c.loader.load(url.clone()).await.map_err(Error::Custom)?.as_ref())?;
                        c.patches.insert(url.clone(), c.cache.load_patch(image.to_rgba()));
                    }
                    Ok(c.patches[&url].clone())
                } else {
                    Err(Error::Syntax("Expected url".into()))
                }?;
                c.take(Token::Comma)?;
                let color = parse_color::<L>(c.tokens.next())?;
                c.take(Token::BraceClose)?;
                Ok(Background::Patch(image, color))
            }
            other => Err(Error::Syntax(format!("{} is not a background type", other))),
        },
        Some(PositionedToken(Token::Color(color), pos)) => Ok(Background::Color(parse_color::<L>(Some(PositionedToken(Token::Color(color), pos)))?)),
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing background".into())),
    }
}

async fn parse_font<I: Iterator<Item = PositionedToken>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Font, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::String(url), _)) => {
            if c.fonts.get(&url).is_none() {
                let font = c
                    .cache
                    .load_font(c.loader.load(url.as_str()).await.map_err(Error::Custom)?);
                c.fonts.insert(url.clone(), font);
            }
            Ok(c.fonts[&url].clone())
        }
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_float<I: Iterator<Item = PositionedToken>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<f32, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::Number(number), _)) => number
            .parse::<f32>()
            .ok()
            .ok_or(Error::Syntax("unable to parse float".into())),
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_rectangle<I: Iterator<Item = PositionedToken>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<Rectangle, Error<L::Error>> {
    let mut result = Rectangle::zero();
    c.take(Token::BraceOpen)?;
    loop {
        match c.tokens.next() {
            Some(PositionedToken(Token::Identifier(field), _)) => {
                c.take(Token::Colon)?;
                match field.as_str() {
                    "left" => result.left = parse_float(c)?,
                    "top" => result.top = parse_float(c)?,
                    "right" => result.right = parse_float(c)?,
                    "bottom" => result.bottom = parse_float(c)?,
                    other => Err(Error::Syntax(format!("unknown field '{}' for rectangle", other)))?,
                }
            }
            Some(PositionedToken(Token::BraceClose, _)) => {
                return Ok(result);
            }
            Some(other) => return Err(Error::Syntax(format!("Unexpected token {:?}", other))),
            None => return Err(Error::Syntax("Unexpected end of file while parsing rectangle".into())),
        }
    }
}

fn parse_text_wrap<I: Iterator<Item = PositionedToken>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<TextWrap, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "no-wrap" => Ok(TextWrap::NoWrap),
            "word-wrap" => Ok(TextWrap::WordWrap),
            "wrap" => Ok(TextWrap::Wrap),
            other => Err(Error::Syntax(format!(
                "{} is not one of no-wrap, word-wrap or wrap",
                other
            ))),
        },
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_align<I: Iterator<Item = PositionedToken>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Align, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "begin" | "left" | "top" => Ok(Align::Begin),
            "center" => Ok(Align::Center),
            "end" | "right" | "bottom" => Ok(Align::End),
            other => Err(Error::Syntax(format!("{} is not one of begin, center or end", other))),
        },
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_size<I: Iterator<Item = PositionedToken>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Size, Error<L::Error>> {
    match c.tokens.next() {
        Some(PositionedToken(Token::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "shrink" => Ok(Size::Shrink),
            "fill" => {
                c.take(Token::BraceOpen)?;
                let size = parse_float(c)?;
                c.take(Token::BraceClose)?;
                Ok(Size::Fill(size as u32))
            }
            other => Err(Error::Syntax(format!("{} is not one of begin, center or end", other))),
        },
        Some(PositionedToken(Token::Number(num), _)) => Ok(Size::Exact(
            num.parse::<f32>()
                .ok()
                .ok_or(Error::Syntax("unable to parse exact size".into()))?,
        )),
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_color<L: Loader>(token: Option<PositionedToken>) -> Result<Color, Error<L::Error>> {
    if let Some(PositionedToken(Token::Color(string), _)) = token {
        let int = u32::from_str_radix(string.as_str(), 16).unwrap();
        match string.len() {
            3 => Ok(Color {
                r: ((int & 0xf00) >> 8) as f32 / 15.0,
                g: ((int & 0x0f0) >> 4) as f32 / 15.0,
                b: ((int & 0x00f) >> 0) as f32 / 15.0,
                a: 1.0,
            }),
            4 => Ok(Color {
                r: ((int & 0xf000) >> 12) as f32 / 15.0,
                g: ((int & 0x0f00) >> 8) as f32 / 15.0,
                b: ((int & 0x00f0) >> 4) as f32 / 15.0,
                a: ((int & 0x000f) >> 0) as f32 / 15.0,
            }),
            6 => Ok(Color {
                r: ((int & 0xff0000) >> 16) as f32 / 255.0,
                g: ((int & 0x00ff00) >> 8) as f32 / 255.0,
                b: ((int & 0x0000ff) >> 0) as f32 / 255.0,
                a: 1.0,
            }),
            8 => Ok(Color {
                r: ((int & 0xff000000) >> 24) as f32 / 255.0,
                g: ((int & 0x00ff0000) >> 16) as f32 / 255.0,
                b: ((int & 0x0000ff00) >> 8) as f32 / 255.0,
                a: ((int & 0x000000ff) >> 0) as f32 / 255.0,
            }),
            _ => Err(Error::ColorParse),
        }
    } else {
        Err(Error::Syntax(format!("Expected 'color', got '{:?}'", token)))
    }
}

impl<E: std::error::Error> From<image::ImageError> for Error<E> {
    fn from(error: image::ImageError) -> Self {
        Error::Image(error)
    }
}

impl<E: std::error::Error> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<E: std::error::Error> std::error::Error for Error<E> {}

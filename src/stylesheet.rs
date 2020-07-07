use std::collections::HashMap;

use crate::draw::{Background, Color};
use crate::layout::{Rectangle, Size};
use crate::text::{Font, TextWrap};
use fnv::FnvHashMap;

#[derive(PartialOrd, Ord, Hash, PartialEq, Eq)]
pub enum ElementKey {
    Button,
    Frame,
    Space,
    Input,
    Text,
    Scroll,
    Toggle,
    Other(String),
}

pub struct Stylesheet {
    background: Option<Background>,
    hover: Option<Background>,
    pressed: Option<Background>,
    clicked: Option<Background>,
    checked: Option<Background>,
    font: Option<Font>,
    color: Option<Color>,
    scrollbar_horizontal: Option<Background>,
    scrollbar_vertical: Option<Background>,
    padding: Option<Rectangle>,
    text_size: Option<f32>,
    text_wrap: Option<TextWrap>,
    width: Option<Size>,
    height: Option<Size>,

    element_rules: FnvHashMap<ElementKey, Stylesheet>,
    class_rules: HashMap<&'static str, Stylesheet>,
}

impl Stylesheet {
    pub fn new(default_font: Font) -> Self {
        Self {
            background: Some(Background::None),
            hover: Some(Background::None),
            pressed: Some(Background::None),
            clicked: Some(Background::None),
            checked: Some(Background::None),
            font: Some(default_font),
            color: Some(Color::white()),
            scrollbar_horizontal: Some(Background::Color(Color::white())),
            scrollbar_vertical: Some(Background::Color(Color::white())),
            padding: Some(Rectangle::zero()),
            text_size: Some(50.0),
            text_wrap: Some(TextWrap::NoWrap),
            width: Some(Size::Fill(1)),
            height: Some(Size::Fill(1)),

            element_rules: Default::default(),
            class_rules: Default::default(),
        }
    }

    fn and_then<'a, F: Clone + Fn(&'a Stylesheet) -> Option<T>, T: 'a>(
        &'a self,
        class: &'static str,
        element: &ElementKey,
        getter: F,
    ) -> Option<T> {
        if let Some(rules) = self.class_rules.get(class) {
            if let Some(result) = rules.and_then(class, element, getter.clone()) {
                return Some(result);
            }
        }

        if let Some(rules) = self.element_rules.get(element) {
            if let Some(result) = rules.and_then(class, element, getter.clone()) {
                return Some(result);
            }
        }

        getter(self)
    }

    pub fn background(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.background.as_ref())
            .unwrap()
    }

    pub fn hover(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.hover.as_ref())
            .unwrap()
    }

    pub fn pressed(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.pressed.as_ref())
            .unwrap()
    }

    pub fn clicked(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.clicked.as_ref())
            .unwrap()
    }

    pub fn checked(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.checked.as_ref())
            .unwrap()
    }

    pub fn font(&self, class: &'static str, element: &ElementKey) -> &Font {
        self.and_then(class, element, |stylesheet| stylesheet.font.as_ref()).unwrap()
    }

    pub fn color(&self, class: &'static str, element: &ElementKey) -> Color {
        self.and_then(class, element, |stylesheet| stylesheet.color).unwrap()
    }

    pub fn scrollbar_horizontal(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.scrollbar_horizontal.as_ref())
            .unwrap()
    }

    pub fn scrollbar_vertical(&self, class: &'static str, element: &ElementKey) -> &Background {
        self.and_then(class, element, |stylesheet| stylesheet.scrollbar_vertical.as_ref())
            .unwrap()
    }

    pub fn padding(&self, class: &'static str, element: &ElementKey) -> &Rectangle {
        self.and_then(class, element, |stylesheet| stylesheet.padding.as_ref())
            .unwrap()
    }

    pub fn text_size(&self, class: &'static str, element: &ElementKey) -> f32 {
        self.and_then(class, element, |stylesheet| stylesheet.text_size).unwrap()
    }

    pub fn text_wrap(&self, class: &'static str, element: &ElementKey) -> TextWrap {
        self.and_then(class, element, |stylesheet| stylesheet.text_wrap).unwrap()
    }

    pub fn width(&self, class: &'static str, element: &ElementKey) -> Size {
        self.and_then(class, element, |stylesheet| stylesheet.width).unwrap()
    }

    pub fn height(&self, class: &'static str, element: &ElementKey) -> Size {
        self.and_then(class, element, |stylesheet| stylesheet.height).unwrap()
    }
}

use std::collections::HashMap;

use futures::future::BoxFuture;
use futures::FutureExt;
use serde::*;

use crate::cache::Cache;
use crate::draw::{Background, Color, Image, Patch};
use crate::layout::{Align, Rectangle, Size};
use crate::Loader;
use crate::text::{Font, TextWrap};

#[derive(Debug)]
pub enum Error {
    Syntax,
    FontMissing,
    ColorParse,
}

#[derive(Deserialize)]
pub enum BackgroundFormat {
    Color(String),
    Image(String, String),
    Patch(String, String),
    None,
}

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

    pub classes: HashMap<String, Stylesheet>,
}

#[derive(Deserialize)]
struct StylesheetFormat {
    background: Option<BackgroundFormat>,
    hover: Option<BackgroundFormat>,
    pressed: Option<BackgroundFormat>,
    disabled: Option<BackgroundFormat>,
    checked: Option<BackgroundFormat>,
    font: Option<String>,
    color: Option<String>,
    scrollbar_horizontal: Option<BackgroundFormat>,
    scrollbar_vertical: Option<BackgroundFormat>,
    padding: Option<Rectangle>,
    text_size: Option<f32>,
    text_wrap: Option<TextWrap>,
    width: Option<Size>,
    height: Option<Size>,
    align_horizontal: Option<Align>,
    align_vertical: Option<Align>,

    #[serde(default)]
    classes: HashMap<String, StylesheetFormat>,
}

impl Stylesheet {
    pub fn new(cache: &mut Cache) -> Self {
        Stylesheet {
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
            classes: Default::default()
        }
    }

    pub async fn load<L, U>(loader: &L, url: U, cache: &mut Cache) -> Result<Self, Error>
    where
        L: Loader,
        U: AsRef<str>,
    {
        let data = loader.load(url).await;
        let imp = ron::de::from_bytes::<StylesheetFormat>(data.as_ref()).unwrap();

        Self::load_inner(
            loader,
            imp,
            cache,
            None,
            &mut HashMap::new(),
            &mut HashMap::new(),
            &mut HashMap::new(),
        )
        .await
    }

    fn load_inner<'a, L>(
        loader: &'a L,
        imp: StylesheetFormat,
        cache: &'a mut Cache,
        inherit: Option<&'a Stylesheet>,
        images: &'a mut HashMap<String, Image>,
        patches: &'a mut HashMap<String, Patch>,
        fonts: &'a mut HashMap<String, Font>,
    ) -> BoxFuture<'a, Result<Self, Error>>
    where
        L: Loader,
    {
        async move {
            let font = if let Some(font) = imp.font.clone() {
                if let Some(font) = fonts.get(&font) {
                    font.clone()
                } else {
                    let result = cache.load_font(loader.load(&font).await);
                    fonts.insert(font.clone(), result.clone());
                    result
                }
            } else {
                inherit.map(|i| i.font.clone()).unwrap_or(cache.load_font(include_bytes!("../default_font.ttf").to_vec()))
            };

            let color = if let Some(color) = imp.color.clone() {
                load_color(color)?
            } else {
                inherit.map(|i| i.color).unwrap_or(Color::white())
            };

            let mut result = Stylesheet {
                background: load_background(
                    imp.background,
                    inherit.map(|i| &i.background),
                    loader,
                    cache,
                    images,
                    patches,
                )
                .await?,
                hover: load_background(imp.hover, inherit.map(|i| &i.hover), loader, cache, images, patches).await?,
                pressed: load_background(imp.pressed, inherit.map(|i| &i.pressed), loader, cache, images, patches)
                    .await?,
                disabled: load_background(imp.disabled, inherit.map(|i| &i.disabled), loader, cache, images, patches)
                    .await?,
                checked: load_background(imp.checked, inherit.map(|i| &i.checked), loader, cache, images, patches)
                    .await?,
                font,
                color,
                scrollbar_horizontal: load_background(
                    imp.scrollbar_horizontal,
                    inherit.map(|i| &i.scrollbar_horizontal),
                    loader,
                    cache,
                    images,
                    patches,
                )
                .await?,
                scrollbar_vertical: load_background(
                    imp.scrollbar_vertical,
                    inherit.map(|i| &i.scrollbar_vertical),
                    loader,
                    cache,
                    images,
                    patches,
                )
                .await?,
                padding: imp.padding.or(inherit.map(|i| i.padding)).unwrap_or(Rectangle::zero()),
                text_size: imp.text_size.or(inherit.map(|i| i.text_size)).unwrap_or(16.0),
                text_wrap: imp
                    .text_wrap
                    .or(inherit.map(|i| i.text_wrap))
                    .unwrap_or(TextWrap::NoWrap),
                width: imp.width.or(inherit.map(|i| i.width)).unwrap_or(Size::Shrink),
                height: imp.height.or(inherit.map(|i| i.height)).unwrap_or(Size::Shrink),
                align_horizontal: imp
                    .align_horizontal
                    .or(inherit.map(|i| i.align_horizontal))
                    .unwrap_or(Align::Begin),
                align_vertical: imp
                    .align_vertical
                    .or(inherit.map(|i| i.align_vertical))
                    .unwrap_or(Align::Begin),

                classes: Default::default(),
            };

            for (class, sheet) in imp.classes.into_iter() {
                let sheet = Stylesheet::load_inner(loader, sheet, cache, Some(&result), images, patches, fonts).await?;
                result.classes.insert(class, sheet);
            }

            Ok(result)
        }
        .boxed()
    }
}

async fn load_background<L: Loader>(
    background: Option<BackgroundFormat>,
    fallback: Option<&Background>,
    loader: &L,
    cache: &mut Cache,
    images: &mut HashMap<String, Image>,
    patches: &mut HashMap<String, Patch>,
) -> Result<Background, Error> {
    Ok(match background {
        Some(BackgroundFormat::Color(hex)) => Background::Color(load_color(hex)?),
        Some(BackgroundFormat::Image(url, hex)) => {
            if images.get(&url).is_none() {
                let image = image::load_from_memory(loader.load(url.clone()).await.as_ref()).unwrap();
                images.insert(url.clone(), cache.load_image(image.to_rgba()));
            }
            Background::Image(images[&url].clone(), load_color(hex)?)
        }
        Some(BackgroundFormat::Patch(url, hex)) => {
            if patches.get(&url).is_none() {
                let image = image::load_from_memory(loader.load(url.clone()).await.as_ref()).unwrap();
                patches.insert(url.clone(), cache.load_patch(image.to_rgba()));
            }
            Background::Patch(patches[&url].clone(), load_color(hex)?)
        }
        Some(BackgroundFormat::None) => Background::None,
        None => fallback.cloned().unwrap_or(Background::None),
    })
}

fn load_color(string: String) -> Result<Color, Error> {
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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error { }
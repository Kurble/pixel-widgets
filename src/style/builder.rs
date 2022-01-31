use image::RgbaImage;

use super::*;
use crate::component::Component;
use anyhow::{Context, Error, Result};
use std::pin::Pin;

type RgbaImageFuture = Pin<Box<dyn Future<Output = Result<RgbaImage>>>>;
type DataFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>>>>>;

/// Builds a style.
#[derive(Default)]
pub struct StyleBuilder {
    pub(crate) images: HashMap<String, RgbaImageFuture>,
    pub(crate) patches: HashMap<String, RgbaImageFuture>,
    pub(crate) fonts: HashMap<String, (RgbaImageFuture, DataFuture)>,
    pub(crate) rule_tree: tree::RuleTreeBuilder,
}

/// Handle to an image in a `StyleBuilder`.
#[derive(Debug, Clone)]
pub struct ImageId(pub(crate) String);
/// Handle to a patch in a `StyleBuilder`.
#[derive(Debug, Clone)]
pub struct PatchId(pub(crate) String);
/// Handle to a font in a `StyleBuilder`.
#[derive(Debug, Clone)]
pub struct FontId(pub(crate) String);

/// Builder that adds style declarations to a selected rule.
pub struct RuleBuilder {
    selector: Vec<Selector>,
    declarations: Vec<Declaration<ImageId, PatchId, FontId>>,
}

impl StyleBuilder {
    fn base(foreground: Color, background: Color, primary: Color) -> Self {
        Self::default()
            .rule(RuleBuilder::new("*").color(foreground))
            .rule(
                RuleBuilder::new("button")
                    .padding_all(5.0)
                    .margin_all(5.0)
                    .background_color(background),
            )
            .rule(RuleBuilder::new("button:hover").background_color(background.blend(primary, 0.5)))
            .rule(RuleBuilder::new("button:pressed").background_color(primary))
            .rule(
                RuleBuilder::new("dropdown")
                    .background_color(background)
                    .color(background.blend(primary, 0.5))
                    .padding_all(5.0)
                    .margin_all(5.0),
            )
            .rule(
                RuleBuilder::new("input")
                    .width(300.0)
                    .background_color(Color::white())
                    .color(Color::black())
                    .padding_all(5.0)
                    .margin_all(5.0),
            )
            .rule(RuleBuilder::new("layers").fill_width().fill_height())
            .rule(
                RuleBuilder::new("menu")
                    .background_color(background)
                    .color(background.blend(primary, 0.5))
                    .padding_all(5.0),
            )
            .rule(RuleBuilder::new("spacer").fill_width().fill_height())
            .rule(
                RuleBuilder::new("window")
                    .background_color(background.blend(foreground, 0.2))
                    .padding_all(2.0),
            )
            .rule(RuleBuilder::new("window > *:nth-child(0)").background_color(background.blend(primary, 0.2)))
    }

    /// Add a rule defined in a [`RuleBuilder`](struct.RuleBuilder.html) to the `StyleBuilder`.
    pub fn rule(mut self, builder: RuleBuilder) -> Self {
        self.rule_tree.insert(builder.selector.as_slice(), builder.declarations);
        self
    }

    /// Prepend the given selector to all rules in this `StyleBuilder`.
    pub fn scope<S: AsRef<str>>(mut self, selector: S) -> Self {
        let mut old = std::mem::take(&mut self.rule_tree);

        let selector = parse_selectors(tokenize(selector.as_ref().to_string()).unwrap()).unwrap();
        if let Some(new_root) = selector.as_slice().last() {
            old.selector = new_root.clone();
        }
        self.rule_tree.select(selector.as_slice()).merge(old);

        self
    }

    /// Merge with another `StyleBuilder`.
    pub fn merge(mut self, builder: StyleBuilder) -> Self {
        self.images.extend(builder.images);
        self.patches.extend(builder.patches);
        self.fonts.extend(builder.fonts);
        self.rule_tree.merge(builder.rule_tree);
        self
    }

    /// Include the scoped style of a `Component` in this `StyleBuilder`.
    pub fn component<C: Component>(mut self) -> Self {
        let mut builder = C::style();
        self.images.extend(builder.images);
        self.patches.extend(builder.patches);
        self.fonts.extend(builder.fonts);
        let name = C::style_scope().to_string();
        builder.rule_tree.selector = Selector::Widget(SelectorWidget::Some(name.clone()));
        self.rule_tree
            .select(&[Selector::Widget(SelectorWidget::Some(name))])
            .merge(builder.rule_tree);
        self
    }

    /// Asynchronously load a stylesheet from a .pwss file. See the [style module documentation](../index.html) on how to write
    /// .pwss files.
    pub async fn from_read_fn<P, R>(path: P, read: R) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
        R: ReadFn,
    {
        let text = String::from_utf8(read.read(path.as_ref()).await?).unwrap();
        Ok(parse(tokenize(text)?, read).await?)
    }

    /// Synchronously load a stylesheet from a .pwss file. See the [style module documentation](../index.html) on how to write
    /// .pwss files.
    pub fn from_file<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        futures::executor::block_on(Self::from_read_fn(path, |path: &Path| {
            std::future::ready(std::fs::read(path))
        }))
    }

    /// Returns an `ImageId` for the `key`.
    /// When the style is built, the image is loaded using the closure.
    pub fn load_image(
        &mut self,
        key: impl Into<String>,
        load: impl FnOnce() -> Result<RgbaImage> + 'static,
    ) -> ImageId {
        self.load_image_async(key, async move { load() })
    }

    /// Returns a `PatchId` for the `key`.
    /// When the style is built, the 9-patch is loaded using the closure.
    pub fn load_patch(
        &mut self,
        key: impl Into<String>,
        load: impl FnOnce() -> Result<RgbaImage> + 'static,
    ) -> PatchId {
        self.load_patch_async(key, async move { load() })
    }

    /// Returns a `FontId` for the `key`.
    /// When the style is built, the font is loaded using the closure.
    /// The closure must return the bytes of a .ttf file.
    pub fn load_font(
        &mut self,
        key: impl Into<String>,
        load_rgba: impl FnOnce() -> Result<RgbaImage> + 'static,
        load_data: impl FnOnce() -> Result<Vec<u8>> + 'static,
    ) -> FontId {
        self.load_font_async(key, async move { load_rgba() }, async move { load_data() })
    }

    /// Returns an `ImageId` for the `key`.
    /// When the style is built, the image is loaded by awaiting the future.
    pub fn load_image_async(
        &mut self,
        key: impl Into<String>,
        fut: impl Future<Output = Result<RgbaImage>> + 'static,
    ) -> ImageId {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.images.entry(key.clone()) {
            v.insert(Box::pin(fut));
        }
        ImageId(key)
    }

    /// Returns a `PatchId` for the `key`.
    /// When the style is built, the 9-patch is loaded by awaiting the future.
    pub fn load_patch_async(
        &mut self,
        key: impl Into<String>,
        fut: impl Future<Output = Result<RgbaImage>> + 'static,
    ) -> PatchId {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.patches.entry(key.clone()) {
            v.insert(Box::pin(fut));
        }
        PatchId(key)
    }

    /// Returns a `FontId` for the `key`.
    /// When the style is built, the font is loaded by awaiting the future.
    /// The future must output the bytes of a .ttf file.
    pub fn load_font_async(
        &mut self,
        key: impl Into<String>,
        fut_rgba: impl Future<Output = Result<RgbaImage>> + 'static,
        fut_data: impl Future<Output = Result<Vec<u8>>> + 'static,
    ) -> FontId {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.fonts.entry(key.clone()) {
            v.insert((Box::pin(fut_rgba), Box::pin(fut_data)));
        }
        FontId(key)
    }

    /// Builds the `Style`. All loading of images, 9 patches and fonts happens in this method.
    /// If any of them fail, an error is returned.
    pub async fn build_async(mut self) -> Result<Style> {
        self = Self::base(Color::white(), Color::rgb(0.3, 0.3, 0.3), Color::blue()).merge(self);

        let mut cache = Cache::new(2048);

        let font_image = image::load_from_memory(include_bytes!("default_font.png"))
            .unwrap()
            .into_rgba8();
        let font = cache
            .load_font(include_bytes!("default_font.json"), font_image)
            .unwrap();

        let mut images = HashMap::new();
        for (key, value) in self.images {
            images.insert(
                key.clone(),
                cache.load_image(
                    value
                        .await
                        .with_context(|| format!("Failed to load image \"{}\": ", key))?,
                ),
            );
        }

        let mut patches = HashMap::new();
        for (key, value) in self.patches {
            patches.insert(
                key.clone(),
                cache.load_patch(
                    value
                        .await
                        .with_context(|| format!("Failed to load 9 patch \"{}\": ", key))?,
                ),
            );
        }

        let mut fonts = HashMap::new();
        for (key, (rgba, data)) in self.fonts {
            let load = async { Result::<_, Error>::Ok(cache.load_font(data.await?, rgba.await?)?) };
            fonts.insert(
                key.clone(),
                load.await
                    .with_context(|| format!("Failed to load font \"{}\": ", key))?,
            );
        }

        Ok(Style {
            cache: Arc::new(Mutex::new(cache)),
            resolved: Default::default(),
            default: Stylesheet {
                background: Background::None,
                font,
                color: Color::white(),
                padding: Rectangle::zero(),
                margin: Rectangle::zero(),
                text_size: 16.0,
                text_border: 0.3,
                text_wrap: TextWrap::NoWrap,
                width: Size::Shrink,
                height: Size::Shrink,
                direction: Direction::LeftToRight,
                align_horizontal: Align::Begin,
                align_vertical: Align::Begin,
                flags: Vec::new(),
            },
            rule_tree: self.rule_tree.build(&images, &patches, &fonts),
        })
    }

    /// Builds the `Style`. All loading of images, 9 patches and fonts happens in this method.
    /// If any of them fail, an error is returned.
    pub fn build(self) -> Result<Style> {
        futures::executor::block_on(self.build_async())
    }
}

impl TryInto<Style> for StyleBuilder {
    type Error = Error;

    fn try_into(self) -> Result<Style> {
        self.build()
    }
}

impl RuleBuilder {
    /// Constructs a new `RuleBuilder` for the given selector.
    /// The selector must follow the same syntax as the [.pwss file format](../index.html).
    ///
    /// Panics if the selector can't be parsed.
    ///
    /// ```rust
    /// use pixel_widgets::prelude::*;
    ///
    /// // Sets the background of the first direct child of any window widget
    /// RuleBuilder::new("window > * :nth-child(0)").background_color(Color::red());
    /// ```
    pub fn new<S: AsRef<str>>(selector: S) -> Self {
        Self {
            selector: parse_selectors(tokenize(selector.as_ref().to_string()).unwrap()).unwrap(),
            declarations: Vec::new(),
        }
    }
    /// Clears the background
    pub fn background_none(mut self) -> Self {
        self.declarations.push(Declaration::BackgroundNone);
        self
    }
    /// Sets the background to a color
    pub fn background_color(mut self, color: Color) -> Self {
        self.declarations.push(Declaration::BackgroundColor(color));
        self
    }
    /// Sets the background to a colored image
    pub fn background_image(mut self, image_data: ImageId, color: Color) -> Self {
        self.declarations.push(Declaration::BackgroundImage(image_data, color));
        self
    }
    /// Sets the background to a colored patch
    pub fn background_patch(mut self, patch: PatchId, color: Color) -> Self {
        self.declarations.push(Declaration::BackgroundPatch(patch, color));
        self
    }
    /// Sets the font
    pub fn font(mut self, value: FontId) -> Self {
        self.declarations.push(Declaration::Font(value));
        self
    }
    /// Sets the foreground color
    pub fn color(mut self, value: Color) -> Self {
        self.declarations.push(Declaration::Color(value));
        self
    }
    /// Sets padding
    pub fn padding(mut self, value: Rectangle) -> Self {
        self.declarations.push(Declaration::Padding(value));
        self
    }
    /// Sets all padding values to the same value
    pub fn padding_all(self, value: f32) -> Self {
        self.padding(Rectangle {
            left: value,
            top: value,
            right: value,
            bottom: value,
        })
    }
    /// Sets horizontal padding values to the same value
    pub fn padding_horizontal(self, value: f32) -> Self {
        self.padding_left(value).padding_right(value)
    }
    /// Sets vertical padding values to the same value
    pub fn padding_vertical(self, value: f32) -> Self {
        self.padding_top(value).padding_bottom(value)
    }
    /// Sets left padding
    pub fn padding_left(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::PaddingLeft(value));
        self
    }
    /// Sets right padding
    pub fn padding_right(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::PaddingRight(value));
        self
    }
    /// Sets top padding
    pub fn padding_top(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::PaddingTop(value));
        self
    }
    /// Sets bottom padding
    pub fn padding_bottom(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::PaddingBottom(value));
        self
    }
    /// Sets the margins
    pub fn margin(mut self, value: Rectangle) -> Self {
        self.declarations.push(Declaration::Margin(value));
        self
    }
    /// Sets all margin values to the same value
    pub fn margin_all(self, value: f32) -> Self {
        self.margin(Rectangle {
            left: value,
            top: value,
            right: value,
            bottom: value,
        })
    }
    /// Sets horizontal margin values to the same value
    pub fn margin_horizontal(self, value: f32) -> Self {
        self.margin_left(value).margin_right(value)
    }
    /// Sets vertical margin values to the same value
    pub fn margin_vertical(self, value: f32) -> Self {
        self.margin_top(value).margin_bottom(value)
    }
    /// Sets the left margin
    pub fn margin_left(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::MarginLeft(value));
        self
    }
    /// Sets the right margin
    pub fn margin_right(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::MarginRight(value));
        self
    }
    /// Sets the top margin
    pub fn margin_top(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::MarginTop(value));
        self
    }
    /// Sets the bottom margin
    pub fn margin_bottom(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::MarginBottom(value));
        self
    }
    /// Sets the text size
    pub fn text_size(mut self, value: f32) -> Self {
        self.declarations.push(Declaration::TextSize(value));
        self
    }
    /// Sets the way text wraps
    pub fn text_wrap(mut self, value: TextWrap) -> Self {
        self.declarations.push(Declaration::TextWrap(value));
        self
    }
    /// Sets the preferred width
    pub fn width(mut self, value: impl Into<Size>) -> Self {
        self.declarations.push(Declaration::Width(value.into()));
        self
    }
    /// Sets the preferred width to Size::Fill(1)
    pub fn fill_width(mut self) -> Self {
        self.declarations.push(Declaration::Width(Size::Fill(1)));
        self
    }
    /// Sets the preferred height
    pub fn height(mut self, value: impl Into<Size>) -> Self {
        self.declarations.push(Declaration::Height(value.into()));
        self
    }
    /// Sets the preferred height to Size::Fill(1)
    pub fn fill_height(mut self) -> Self {
        self.declarations.push(Declaration::Height(Size::Fill(1)));
        self
    }
    /// Sets the direction for layouting
    pub fn layout_direction(mut self, value: Direction) -> Self {
        self.declarations.push(Declaration::LayoutDirection(value));
        self
    }
    /// Sets the horizontal alignment
    pub fn align_horizontal(mut self, value: Align) -> Self {
        self.declarations.push(Declaration::AlignHorizontal(value));
        self
    }
    /// Sets the vertical alignment
    pub fn align_vertical(mut self, value: Align) -> Self {
        self.declarations.push(Declaration::AlignVertical(value));
        self
    }
    /// Adds a flag to the stylesheet
    pub fn add_flag(mut self, value: String) -> Self {
        self.declarations.push(Declaration::AddFlag(value));
        self
    }
    /// Removes a flag from the stylesheet
    pub fn remove_flag(mut self, value: String) -> Self {
        self.declarations.push(Declaration::RemoveFlag(value));
        self
    }
}

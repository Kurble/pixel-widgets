use image::RgbaImage;

use super::*;
use crate::component::Component;
use anyhow::*;

/// Builds a style.
#[derive(Default)]
pub struct StyleBuilder {
    pub(crate) images: HashMap<String, RgbaImage>,
    pub(crate) patches: HashMap<String, RgbaImage>,
    pub(crate) fonts: HashMap<String, Vec<u8>>,
    pub(crate) rule_tree: tree::RuleTreeBuilder,
}

/// Handle to an image in a `StyleBuilder`.
#[derive(Debug)]
pub struct ImageId(pub(crate) String);
/// Handle to a patch in a `StyleBuilder`.
#[derive(Debug)]
pub struct PatchId(pub(crate) String);
/// Handle to a font in a `StyleBuilder`.
#[derive(Debug)]
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

    /// Puts an `RgbaImage` in the style cache.
    pub fn load_image(&mut self, key: impl Into<String>, image: impl FnOnce() -> Result<RgbaImage>) -> Result<ImageId> {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.images.entry(key.clone()) {
            v.insert(image()?);
        }
        Ok(ImageId(key))
    }

    /// Puts a 9 patch `RgbaImage` in the style cache.
    pub fn load_patch(&mut self, key: impl Into<String>, patch: impl FnOnce() -> Result<RgbaImage>) -> Result<PatchId> {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.patches.entry(key.clone()) {
            v.insert(patch()?);
        }
        Ok(PatchId(key))
    }

    /// Puts a font loaded from a .ttf file in the style cache.
    pub fn load_font(&mut self, key: impl Into<String>, font: impl FnOnce() -> Result<Vec<u8>>) -> Result<FontId> {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.fonts.entry(key.clone()) {
            v.insert(font()?);
        }
        Ok(FontId(key))
    }

    /// Puts an `RgbaImage` in the style cache.
    pub async fn load_image_async(
        &mut self,
        key: impl Into<String>,
        image: impl Future<Output = Result<RgbaImage>>,
    ) -> Result<ImageId> {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.images.entry(key.clone()) {
            v.insert(image.await?);
        }
        Ok(ImageId(key))
    }

    /// Puts a 9 patch `RgbaImage` in the style cache.
    pub async fn load_patch_async(
        &mut self,
        key: impl Into<String>,
        patch: impl Future<Output = Result<RgbaImage>>,
    ) -> Result<PatchId> {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.patches.entry(key.clone()) {
            v.insert(patch.await?);
        }
        Ok(PatchId(key))
    }

    /// Puts a font loaded from a .ttf file in the style cache.
    pub async fn load_font_async(
        &mut self,
        key: impl Into<String>,
        font: impl Future<Output = Result<Vec<u8>>>,
    ) -> Result<FontId> {
        let key = key.into();
        if let std::collections::hash_map::Entry::Vacant(v) = self.fonts.entry(key.clone()) {
            v.insert(font.await?);
        }
        Ok(FontId(key))
    }

    /// Creates a `DeclarationBuilder` for the supplied selector. This can be used to add
    ///  style declarations to the selected widgets.
    /// The `DeclarationBuilder` will automatically apply the declarations to this `StyleBuilder`
    ///  when it is dropped.
    pub fn rule(mut self, builder: RuleBuilder) -> Self {
        self.rule_tree.insert(builder.selector.as_slice(), builder.declarations);
        self
    }

    /// Put the current contents behind a scope
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

    /// Include the scoped style of a `Component`.
    pub fn component<C: Component>(mut self) -> Self {
        let mut builder = C::style();
        self.images.extend(builder.images);
        self.patches.extend(builder.patches);
        self.fonts.extend(builder.fonts);
        let name = std::any::type_name::<C>().to_string();
        builder.rule_tree.selector = Selector::Widget(SelectorWidget::Some(name.clone()));
        self.rule_tree
            .select(&[Selector::Widget(SelectorWidget::Some(name))])
            .merge(builder.rule_tree);
        self
    }

    /// Builds the `Style`.
    pub fn build(mut self) -> Style {
        let mut cache = Cache::new(512, 0);

        self = Self::base(Color::white(), Color::rgb(0.3, 0.3, 0.3), Color::blue()).merge(self);

        let font = cache.load_font(include_bytes!("default_font.ttf").to_vec());

        let images = self
            .images
            .into_iter()
            .map(|(key, value)| (key, cache.load_image(value)))
            .collect::<HashMap<String, ImageData>>();
        let patches = self
            .patches
            .into_iter()
            .map(|(key, value)| (key, cache.load_patch(value)))
            .collect::<HashMap<String, Patch>>();
        let fonts = self
            .fonts
            .into_iter()
            .map(|(key, value)| (key, cache.load_font(value)))
            .collect::<HashMap<String, Font>>();

        let mut rule_tree = tree::RuleTree::default();
        self.rule_tree.flatten(&mut rule_tree, &images, &patches, &fonts);

        Style {
            cache: Arc::new(Mutex::new(cache)),
            resolved: Default::default(),
            default: Stylesheet {
                background: Background::None,
                font,
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
            rule_tree,
        }
    }
}

impl Into<Style> for StyleBuilder {
    fn into(self) -> Style {
        self.build()
    }
}

impl RuleBuilder {
    /// Constructs a new `RuleBuilder` for a selector
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
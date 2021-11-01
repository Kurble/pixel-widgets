use image::RgbaImage;

use super::*;

/// Builds a style.
pub struct StyleBuilder {
    // maybe, instead of a cache, I could build a list of data to build later?
    pub(crate) cache: Cache,
    pub(crate) rule_tree: tree::RuleTreeBuilder,
}

/// Builder that adds style declarations to a selected rule.
pub struct RuleBuilder {
    selector: Vec<Selector>,
    declarations: Vec<Declaration>,
}

impl StyleBuilder {
    /// Returns a new `StyleBuilder`.
    pub fn new() -> Self {
        Self::new_themed(Color::white(), Color::rgb(0.3, 0.3, 0.3), Color::blue())
    }
    /// Returns a new `StyleBuilder`.
    pub fn new_themed(foreground: Color, background: Color, primary: Color) -> Self {
        Self {
            cache: Cache::new(512, 0),
            rule_tree: tree::RuleTreeBuilder::new(Selector::Widget("*".into())),
        }
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
        .rule(RuleBuilder::new("layers").width(Size::Fill(1)).height(Size::Fill(1)))
        .rule(
            RuleBuilder::new("menu")
                .background_color(background)
                .color(background.blend(primary, 0.5))
                .padding_all(5.0),
        )
        .rule(RuleBuilder::new("spacer").width(Size::Fill(1)).height(Size::Fill(1)))
        .rule(
            RuleBuilder::new("window")
                .background_color(background.blend(foreground, 0.2))
                .padding_all(2.0),
        )
        .rule(RuleBuilder::new("window > *:nth-child(0)").background_color(background.blend(primary, 0.2)))
    }

    /// Puts an `RgbaImage` in the style cache.
    pub fn load_image(&mut self, image: RgbaImage) -> ImageData {
        self.cache.load_image(image)
    }

    /// Puts a 9 patch `RgbaImage` in the style cache.
    pub fn load_patch(&mut self, image: RgbaImage) -> Patch {
        self.cache.load_patch(image)
    }

    /// Puts a font loaded from a .ttf file in the style cache.
    pub fn load_font(&mut self, font: impl Into<Vec<u8>>) -> Font {
        self.cache.load_font(font)
    }

    /// Creates a `DeclarationBuilder` for the supplied selector. This can be used to add
    ///  style declarations to the selected widgets.
    /// The `DeclarationBuilder` will automatically apply the declarations to this `StyleBuilder`
    ///  when it is dropped.
    pub fn rule(mut self, builder: RuleBuilder) -> Self {
        self.rule_tree.insert(builder.selector.as_slice(), builder.declarations);
        self
    }

    /// Puts a StyleBuilder in a scope.
    pub fn scope<S: AsRef<str>>(self, _selector: S, _builder: StyleBuilder) -> Self {
        todo!()
    }

    /// Builds the `Style`.
    pub fn build(mut self) -> Arc<Style> {
        let font = self.cache.load_font(include_bytes!("default_font.ttf").to_vec());
        Arc::new(Style {
            cache: Arc::new(Mutex::new(self.cache)),
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
            rule_tree: self.rule_tree.into(),
        })
    }
}

impl Default for StyleBuilder {
    fn default() -> Self {
        Self::new()
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
    /// Sets the background
    pub fn background(mut self, value: Background) -> Self {
        self.declarations.push(Declaration::Background(value));
        self
    }
    /// Sets the background to a color
    pub fn background_color(self, color: Color) -> Self {
        self.background(Background::Color(color))
    }
    /// Sets the background to a colored image
    pub fn background_image(self, image_data: ImageData, color: Color) -> Self {
        self.background(Background::Image(image_data, color))
    }
    /// Sets the background to a colored patch
    pub fn background_patch(self, patch: Patch, color: Color) -> Self {
        self.background(Background::Patch(patch, color))
    }
    /// Sets the font
    pub fn font(mut self, value: Font) -> Self {
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
    /// Sets the preferred height
    pub fn height(mut self, value: impl Into<Size>) -> Self {
        self.declarations.push(Declaration::Height(value.into()));
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

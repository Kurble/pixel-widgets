use super::*;
use anyhow::*;

struct LoadContext<'a, I: Iterator<Item = Token>, R: ReadFn> {
    loader: R,
    tokens: TokenProvider<I>,
    cache: &'a mut Cache,
    images: &'a mut HashMap<String, ImageData>,
    patches: &'a mut HashMap<String, Patch>,
    fonts: &'a mut HashMap<String, Font>,
}

struct TokenProvider<I: Iterator<Item = Token>> {
    tokens: Peekable<I>,
}

impl<I: Iterator<Item = Token>> TokenProvider<I> {
    pub fn next(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    pub fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    pub fn take(&mut self, token: TokenValue) -> anyhow::Result<Token> {
        let Token(value, pos) = self.tokens.next().ok_or_else(|| anyhow!("EOF"))?;
        if token == value {
            Ok(Token(value, pos))
        } else {
            Err(anyhow!("Expected '{:?}' at {}", token, pos))
        }
    }

    pub fn take_identifier(&mut self) -> anyhow::Result<(String, TokenPos)> {
        match self.tokens.next().ok_or_else(|| anyhow!("EOF"))? {
            Token(TokenValue::Iden(id), pos) => Ok((id, pos)),
            Token(_, pos) => Err(anyhow!("Expected 'Identifier' at {}", pos)),
        }
    }
}

pub async fn parse(tokens: Vec<Token>, loader: impl ReadFn) -> anyhow::Result<Arc<Style>> {
    let mut builder = Style::builder();

    let mut images = HashMap::new();
    let mut patches = HashMap::new();
    let mut fonts = HashMap::new();
    let mut context = LoadContext {
        loader,
        tokens: TokenProvider {
            tokens: tokens.into_iter().peekable(),
        },
        cache: &mut builder.cache,
        images: &mut images,
        patches: &mut patches,
        fonts: &mut fonts,
    };

    //let mut rule_tree = RuleTreeBuilder::new(Selector::Widget(SelectorWidget::Any));
    while context.tokens.peek().is_some() {
        let (selectors, rules) = parse_rule(&mut context).await?;
        builder.rule_tree.insert(selectors, rules);
    }

    Ok(builder.build())
}

pub fn parse_selectors(tokens: Vec<Token>) -> anyhow::Result<Vec<Selector>> {
    let mut p = TokenProvider {
        tokens: tokens.into_iter().peekable(),
    };
    let mut result = Vec::new();
    while p.peek().is_some() {
        result.push(parse_selector(&mut p)?);
    }
    Ok(result)
}

async fn parse_rule<I: Iterator<Item = Token>, L: ReadFn>(
    c: &mut LoadContext<'_, I, L>,
) -> anyhow::Result<(Vec<Selector>, Vec<Declaration>)> {
    let mut selectors = Vec::new();
    let mut declarations = Vec::new();
    loop {
        if let Token(TokenValue::BraceOpen, _) = c.tokens.peek().ok_or_else(|| anyhow!("EOF"))? {
            c.tokens.next();
            loop {
                if let Some(&Token(TokenValue::BraceClose, _)) = c.tokens.peek() {
                    break;
                } else {
                    declarations.push(parse_declaration(c).await?);
                }
            }
            c.tokens.take(TokenValue::BraceClose)?;
            return Ok((selectors, declarations));
        } else {
            selectors.push(parse_selector(&mut c.tokens)?);
        }
    }
}

async fn parse_declaration<I: Iterator<Item = Token>, L: ReadFn>(
    c: &mut LoadContext<'_, I, L>,
) -> anyhow::Result<Declaration> {
    let result = match c.tokens.next() {
        Some(Token(TokenValue::Iden(key), _)) => {
            c.tokens.take(TokenValue::Colon)?;
            match key.as_str() {
                "background" => Ok(Declaration::Background(parse_background(c).await?)),
                "font" => Ok(Declaration::Font(parse_font(c).await?)),
                "color" => Ok(Declaration::Color(parse_color(&mut c.tokens)?)),
                "padding" => Ok(Declaration::Padding(parse_rectangle(&mut c.tokens)?)),
                "padding-left" => Ok(Declaration::PaddingLeft(parse_float(&mut c.tokens)?)),
                "padding-right" => Ok(Declaration::PaddingRight(parse_float(&mut c.tokens)?)),
                "padding-top" => Ok(Declaration::PaddingTop(parse_float(&mut c.tokens)?)),
                "padding-bottom" => Ok(Declaration::PaddingBottom(parse_float(&mut c.tokens)?)),
                "margin" => Ok(Declaration::Margin(parse_rectangle(&mut c.tokens)?)),
                "margin-left" => Ok(Declaration::MarginLeft(parse_float(&mut c.tokens)?)),
                "margin-right" => Ok(Declaration::MarginRight(parse_float(&mut c.tokens)?)),
                "margin-top" => Ok(Declaration::MarginTop(parse_float(&mut c.tokens)?)),
                "margin-bottom" => Ok(Declaration::MarginBottom(parse_float(&mut c.tokens)?)),
                "text-size" => Ok(Declaration::TextSize(parse_float(&mut c.tokens)?)),
                "text-wrap" => Ok(Declaration::TextWrap(parse_text_wrap(&mut c.tokens)?)),
                "width" => Ok(Declaration::Width(parse_size(&mut c.tokens)?)),
                "height" => Ok(Declaration::Height(parse_size(&mut c.tokens)?)),
                "layout-direction" => Ok(Declaration::LayoutDirection(parse_direction(&mut c.tokens)?)),
                "align-horizontal" => Ok(Declaration::AlignHorizontal(parse_align(&mut c.tokens)?)),
                "align-vertical" => Ok(Declaration::AlignVertical(parse_align(&mut c.tokens)?)),
                flag => {
                    let (id, pos) = c.tokens.take_identifier()?;
                    match id.as_str() {
                        "true" => Ok(Declaration::AddFlag(flag.to_string())),
                        "false" => Ok(Declaration::RemoveFlag(flag.to_string())),
                        _ => Err(anyhow!("Flag values must be either `true` or `false` at {}", pos)),
                    }
                }
            }
        }
        Some(Token(_, pos)) => Err(anyhow!("Expected <property> at {}", pos)),
        None => Err(anyhow!("EOF")),
    }?;
    c.tokens.take(TokenValue::Semi)?;
    Ok(result)
}

async fn parse_background<I: Iterator<Item = Token>, L: ReadFn>(
    c: &mut LoadContext<'_, I, L>,
) -> anyhow::Result<Background> {
    match c.tokens.peek().cloned().ok_or_else(|| anyhow!("EOF"))? {
        Token(TokenValue::Iden(ty), pos) => {
            c.tokens.next();
            match ty.to_lowercase().as_str() {
                "none" => Ok(Background::None),
                "image" => {
                    c.tokens.take(TokenValue::ParenOpen)?;
                    let image = match c.tokens.next() {
                        Some(Token(TokenValue::Path(url), _)) => {
                            if c.images.get(&url).is_none() {
                                let image =
                                    image::load_from_memory(c.loader.read(Path::new(url.as_str())).await?.as_ref())?;
                                c.images.insert(url.clone(), c.cache.load_image(image.to_rgba8()));
                            }
                            Ok(c.images[&url].clone())
                        }
                        Some(Token(_, pos)) => Err(anyhow!("Expected <url> at {}", pos)),
                        None => Err(anyhow!("EOF")),
                    }?;
                    c.tokens.take(TokenValue::Comma)?;
                    let color = parse_color(&mut c.tokens)?;
                    c.tokens.take(TokenValue::ParenClose)?;
                    Ok(Background::Image(image, color))
                }
                "patch" => {
                    c.tokens.take(TokenValue::ParenOpen)?;
                    let image = match c.tokens.next() {
                        Some(Token(TokenValue::Path(url), _)) => {
                            if c.patches.get(&url).is_none() {
                                let image =
                                    image::load_from_memory(c.loader.read(Path::new(url.as_str())).await?.as_ref())?;
                                c.patches.insert(url.clone(), c.cache.load_patch(image.to_rgba8()));
                            }
                            Ok(c.patches[&url].clone())
                        }
                        Some(Token(_, pos)) => Err(anyhow!("Expected url at {}", pos)),
                        None => Err(anyhow!("EOF")),
                    }?;
                    c.tokens.take(TokenValue::Comma)?;
                    let color = parse_color(&mut c.tokens)?;
                    c.tokens.take(TokenValue::ParenClose)?;
                    Ok(Background::Patch(image, color))
                }
                _ => Err(anyhow!("Expected `image`, `patch` or `none` at {}", pos)),
            }
        }
        Token(TokenValue::Color(_), _) => Ok(Background::Color(parse_color(&mut c.tokens)?)),
        Token(TokenValue::Path(url), _) => {
            c.tokens.next();
            if url.ends_with(".9.png") {
                if c.patches.get(&url).is_none() {
                    let image = image::load_from_memory(c.loader.read(Path::new(url.as_str())).await?.as_ref())?;
                    c.patches.insert(url.clone(), c.cache.load_patch(image.to_rgba8()));
                }
                Ok(Background::Patch(c.patches[&url].clone(), Color::white()))
            } else {
                if c.images.get(&url).is_none() {
                    let image = image::load_from_memory(c.loader.read(Path::new(url.as_str())).await?.as_ref())?;
                    c.images.insert(url.clone(), c.cache.load_image(image.to_rgba8()));
                }
                Ok(Background::Image(c.images[&url].clone(), Color::white()))
            }
        }
        Token(_, pos) => Err(anyhow!(
            "Expected `none`, `image(<url>, <color>)`, `patch(<url>, <color>)`, <color> or <url> at {}",
            pos,
        )),
    }
}

async fn parse_font<I: Iterator<Item = Token>, L: ReadFn>(c: &mut LoadContext<'_, I, L>) -> anyhow::Result<Font> {
    match c.tokens.next() {
        Some(Token(TokenValue::Path(url), _)) => {
            if c.fonts.get(&url).is_none() {
                let font = c.loader.read(Path::new(url.as_str())).await?;
                let font = c.cache.load_font(font);
                c.fonts.insert(url.clone(), font);
            }
            Ok(c.fonts[&url].clone())
        }
        Some(Token(_, pos)) => Err(anyhow!("Expected <url> at {}", pos)),
        None => Err(anyhow!("EOF")),
    }
}

fn parse_selector<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> anyhow::Result<Selector> {
    match c.tokens.next().ok_or_else(|| anyhow!("EOF"))? {
        Token(TokenValue::Star, _) => Ok(Selector::Widget(SelectorWidget::Any)),
        Token(TokenValue::Dot, _) => Ok(Selector::Class(c.take_identifier()?.0)),
        Token(TokenValue::Iden(widget), _) => Ok(Selector::Widget(SelectorWidget::Some(widget))),
        Token(TokenValue::Gt, _) => Ok(Selector::WidgetDirectChild(parse_widget(c)?)),
        Token(TokenValue::Plus, _) => Ok(Selector::WidgetDirectAfter(parse_widget(c)?)),
        Token(TokenValue::Tilde, _) => Ok(Selector::WidgetAfter(parse_widget(c)?)),
        Token(TokenValue::Colon, _) => {
            let (id, _pos) = c.take_identifier()?;
            match id.as_str() {
                "nth-child-mod" => {
                    c.take(TokenValue::ParenOpen)?;
                    let numerator = parse_usize(c)?;
                    c.take(TokenValue::Comma)?;
                    let denominator = parse_usize(c)?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(Selector::NthMod(numerator, denominator))
                }
                "nth-last-child-mod" => {
                    c.take(TokenValue::ParenOpen)?;
                    let numerator = parse_usize(c)?;
                    c.take(TokenValue::Comma)?;
                    let denominator = parse_usize(c)?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(Selector::NthLastMod(numerator, denominator))
                }
                "nth-child" => {
                    c.take(TokenValue::ParenOpen)?;
                    let result = match c.tokens.next().ok_or_else(|| anyhow!("EOF"))? {
                        Token(TokenValue::Iden(special), pos) => match special.as_str() {
                            "odd" => Ok(Selector::NthMod(1, 2)),
                            "even" => Ok(Selector::NthMod(0, 2)),
                            _ => Err(anyhow!("Expected 'odd', 'even' or <number> at {}", pos)),
                        },
                        Token(TokenValue::Number(number), pos) => Ok(Selector::Nth(
                            number.parse::<usize>().map_err(|err| anyhow!("{} at {}", err, pos))?,
                        )),
                        Token(_, pos) => Err(anyhow!("Expected 'odd', 'even' or <number> at {}", pos)),
                    }?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(result)
                }
                "nth-last-child" => {
                    c.take(TokenValue::ParenOpen)?;
                    let result = match c.tokens.next().ok_or_else(|| anyhow!("EOF"))? {
                        Token(TokenValue::Iden(special), pos) => match special.as_str() {
                            "odd" => Ok(Selector::NthLastMod(1, 2)),
                            "even" => Ok(Selector::NthLastMod(0, 2)),
                            _ => Err(anyhow!("Expected 'odd', 'even' or <number> at {}", pos)),
                        },
                        Token(TokenValue::Number(number), pos) => Ok(Selector::NthLast(
                            number.parse::<usize>().map_err(|err| anyhow!("{} at {}", err, pos))?,
                        )),
                        Token(_, pos) => Err(anyhow!("Expected 'odd', 'even' or <number> at {}", pos)),
                    }?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(result)
                }
                "first-child" => Ok(Selector::Nth(0)),
                "last-child" => Ok(Selector::NthLast(0)),
                "only-child" => Ok(Selector::OnlyChild),
                "not" => {
                    c.take(TokenValue::ParenOpen)?;
                    let inner = parse_selector(c)?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(Selector::Not(Box::new(inner)))
                }
                "hover" => Ok(Selector::State(StyleState::Hover)),
                "pressed" => Ok(Selector::State(StyleState::Pressed)),
                "checked" => Ok(Selector::State(StyleState::Checked)),
                "disabled" => Ok(Selector::State(StyleState::Disabled)),
                "open" => Ok(Selector::State(StyleState::Open)),
                "closed" => Ok(Selector::State(StyleState::Closed)),
                "drag" => Ok(Selector::State(StyleState::Drag)),
                "drop" => Ok(Selector::State(StyleState::Drop)),
                state => Ok(Selector::State(StyleState::Custom(state.to_string()))),
            }
        }
        Token(_, pos) => Err(anyhow!("expected `<selector>` at {}", pos)),
    }
}

fn parse_widget<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<SelectorWidget> {
    match c.next().ok_or_else(|| anyhow!("EOF"))? {
        Token(TokenValue::Star, _) => Ok(SelectorWidget::Any),
        Token(TokenValue::Iden(widget), _) => Ok(SelectorWidget::Some(widget)),
        Token(_, pos) => Err(anyhow!("Expected '*' or 'identifier' at {}", pos)),
    }
}

fn parse_float<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<f32> {
    match c.next() {
        Some(Token(TokenValue::Number(number), pos)) => {
            number.parse::<f32>().map_err(|err| anyhow!("{} at {}", err, pos))
        }
        Some(Token(_, pos)) => Err(anyhow!("Expected <number> at {}", pos)),
        None => Err(anyhow!("EOF")),
    }
}

fn parse_usize<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<usize> {
    match c.next() {
        Some(Token(TokenValue::Number(number), pos)) => {
            number.parse::<usize>().map_err(|err| anyhow!("{} at {}", err, pos))
        }
        Some(Token(_, pos)) => Err(anyhow!("Expected <integer> at {}", pos)),
        None => Err(anyhow!("EOF")),
    }
}

fn parse_rectangle<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<Rectangle> {
    let mut numbers = Vec::new();

    while let Token(TokenValue::Number(_), _) = c.peek().ok_or_else(|| anyhow!("EOF"))? {
        numbers.push(parse_float(c)?);
    }

    match numbers.len() {
        0 => Ok(Rectangle::zero()),
        1 => Ok(Rectangle {
            top: numbers[0],
            right: numbers[0],
            bottom: numbers[0],
            left: numbers[0],
        }),
        2 => Ok(Rectangle {
            top: numbers[0],
            right: numbers[1],
            bottom: numbers[0],
            left: numbers[1],
        }),
        3 => Ok(Rectangle {
            top: numbers[0],
            right: numbers[1],
            bottom: numbers[2],
            left: numbers[1],
        }),
        _ => Ok(Rectangle {
            top: numbers[0],
            right: numbers[1],
            bottom: numbers[2],
            left: numbers[3],
        }),
    }
}

fn parse_text_wrap<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<TextWrap> {
    match c.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "no-wrap" => Ok(TextWrap::NoWrap),
            "word-wrap" => Ok(TextWrap::WordWrap),
            "wrap" => Ok(TextWrap::Wrap),
            _ => Err(anyhow!("Expected `no-wrap`, `word-wrap` or `wrap` at {}", pos)),
        },
        Some(Token(_, pos)) => Err(anyhow!("Expected `no-wrap`, `word-wrap` or `wrap` at {}", pos)),
        None => Err(anyhow!("EOF")),
    }
}

fn parse_direction<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<Direction> {
    match c.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "top-to-bottom" => Ok(Direction::TopToBottom),
            "left-to-right" => Ok(Direction::LeftToRight),
            "right-to-left" => Ok(Direction::RightToLeft),
            "bottom-to-top" => Ok(Direction::BottomToTop),
            _ => Err(anyhow!(
                "Expected `top-to-bottom`, `left-to-right`, `right-to-left` or `bottom-to-top` at {}",
                pos,
            )),
        },
        Some(Token(_, pos)) => Err(anyhow!(
            "Expected `top-to-bottom`, `left-to-right`, `right-to-left` or `bottom-to-top` at {}",
            pos,
        )),
        None => Err(anyhow!("EOF")),
    }
}

fn parse_align<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<Align> {
    match c.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "begin" | "left" | "top" => Ok(Align::Begin),
            "center" => Ok(Align::Center),
            "end" | "right" | "bottom" => Ok(Align::End),
            _ => Err(anyhow!("Expected `begin`, `center` or `end` at {}", pos)),
        },
        Some(Token(_, pos)) => Err(anyhow!("Expected `begin`, `center` or `end` at {}", pos)),
        None => Err(anyhow!("EOF")),
    }
}

fn parse_size<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<Size> {
    match c.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "shrink" => Ok(Size::Shrink),
            "fill" => {
                c.take(TokenValue::ParenOpen)?;
                let size = parse_usize(c)?;
                c.take(TokenValue::ParenClose)?;
                Ok(Size::Fill(size as u32))
            }
            _ => Err(anyhow!("Expected `shrink`, `fill(<integer>)` or <number> at {}", pos,)),
        },
        Some(Token(TokenValue::Number(num), pos)) => Ok(Size::Exact(
            num.parse::<f32>().map_err(|err| anyhow!("{} at {}", err, pos))?,
        )),
        Some(Token(_, pos)) => Err(anyhow!("Expected `shrink`, `fill(<integer>)` or <number> at {}", pos,)),
        None => Err(anyhow!("EOF")),
    }
}

#[allow(clippy::identity_op)] // to keep the code clean and consistent
fn parse_color<I: Iterator<Item = Token>>(c: &mut TokenProvider<I>) -> Result<Color> {
    match c.next().ok_or_else(|| anyhow!("EOF"))? {
        Token(TokenValue::Color(string), pos) => {
            let int = u32::from_str_radix(string.as_str(), 16).map_err(|err| anyhow!("{} at {}", err, pos))?;
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
                _ => Err(anyhow!(
                    "Color values must match one of the following hex patterns: #rgb, #rgba, #rrggbb or #rrggbbaa at {}",
                    pos,
                )),
            }
        }
        Token(_, pos) => Err(anyhow!("Expected <color> at {}", pos)),
    }
}

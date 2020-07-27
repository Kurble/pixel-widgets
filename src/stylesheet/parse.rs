use super::tree::RuleTreeBuilder;
use super::*;

struct LoadContext<'a, I: Iterator<Item = Token>, L: Loader> {
    loader: &'a L,
    tokens: Peekable<I>,
    cache: &'a mut Cache,
    images: &'a mut HashMap<String, Image>,
    patches: &'a mut HashMap<String, Patch>,
    fonts: &'a mut HashMap<String, Font>,
}

impl<I: Iterator<Item = Token>, L: Loader> LoadContext<'_, I, L> {
    pub fn take(&mut self, token: TokenValue) -> Result<Token, Error<L::Error>> {
        let Token(value, pos) = self.tokens.next().ok_or(Error::Eof)?;
        if token == value {
            Ok(Token(value, pos))
        } else {
            Err(Error::Syntax(format!("Expected '{:?}'", token), pos))
        }
    }

    pub fn take_identifier(&mut self) -> Result<(String, TokenPos), Error<L::Error>> {
        match self.tokens.next().ok_or(Error::Eof)? {
            Token(TokenValue::Iden(id), pos) => Ok((id, pos)),
            Token(_, pos) => Err(Error::Syntax("Expected 'Identifier'".into(), pos)),
        }
    }
}

pub async fn parse<L: Loader>(tokens: Vec<Token>, loader: &L, cache: &mut Cache) -> Result<Style, Error<L::Error>> {
    let mut result = Style::new(&mut *cache);

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

    let mut rule_tree = RuleTreeBuilder::new(Selector::Widget(SelectorWidget::Any));
    while let Some(_) = context.tokens.peek() {
        let (selectors, rules) = parse_rule(&mut context).await?;
        rule_tree.insert(selectors, rules);
    }

    result.rule_tree = rule_tree.into();

    Ok(result)
}

async fn parse_rule<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<(Vec<Selector>, Vec<Declaration>), Error<L::Error>> {
    let mut selectors = Vec::new();
    let mut declarations = Vec::new();
    loop {
        if let &Token(TokenValue::BraceOpen, _) = c.tokens.peek().ok_or(Error::Eof)? {
            c.tokens.next();
            loop {
                if let Some(&Token(TokenValue::BraceClose, _)) = c.tokens.peek() {
                    break;
                } else {
                    declarations.push(parse_declaration(c).await?);
                }
            }
            c.take(TokenValue::BraceClose)?;
            return Ok((selectors, declarations));
        } else {
            selectors.push(parse_selector(c)?);
        }
    }
}

async fn parse_declaration<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Declaration, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Iden(key), pos)) => {
            c.take(TokenValue::Colon)?;
            match key.as_str() {
                "background" => Ok(Declaration::Background(parse_background(c).await?)),
                "font" => Ok(Declaration::Font(parse_font(c).await?)),
                "color" => Ok(Declaration::Color(parse_color(c)?)),
                "scrollbar-horizontal" => Ok(Declaration::ScrollbarHorizontal(parse_background(c).await?)),
                "scrollbar-vertical" => Ok(Declaration::ScrollbarVertical(parse_background(c).await?)),
                "padding" => Ok(Declaration::Padding(parse_rectangle(c)?)),
                "text-size" => Ok(Declaration::TextSize(parse_float(c)?)),
                "text-wrap" => Ok(Declaration::TextWrap(parse_text_wrap(c)?)),
                "width" => Ok(Declaration::Width(parse_size(c)?)),
                "height" => Ok(Declaration::Height(parse_size(c)?)),
                "align-horizontal" => Ok(Declaration::AlignHorizontal(parse_align(c)?)),
                "align-vertical" => Ok(Declaration::AlignVertical(parse_align(c)?)),
                unrecognized => Err(Error::Syntax(format!("Rule '{}' not recognized", unrecognized), pos)),
            }
        }
        Some(Token(_, pos)) => Err(Error::Syntax("Expected <property>".into(), pos)),
        None => Err(Error::Eof),
    }
}

async fn parse_background<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Background, Error<L::Error>> {
    match c.tokens.peek().cloned().ok_or(Error::Eof)? {
        Token(TokenValue::Iden(ty), pos) => {
            c.tokens.next();
            match ty.to_lowercase().as_str() {
                "none" => Ok(Background::None),
                "image" => {
                    c.take(TokenValue::ParenOpen)?;
                    let image = match c.tokens.next() {
                        Some(Token(TokenValue::Path(url), _)) => {
                            if c.images.get(&url).is_none() {
                                let image = image::load_from_memory(
                                    c.loader.load(url.clone()).await.map_err(Error::Io)?.as_ref(),
                                )?;
                                c.images.insert(url.clone(), c.cache.load_image(image.to_rgba()));
                            }
                            Ok(c.images[&url].clone())
                        }
                        Some(Token(_, pos)) => Err(Error::Syntax("Expected <url>".into(), pos)),
                        None => Err(Error::Eof),
                    }?;
                    c.take(TokenValue::Comma)?;
                    let color = parse_color(c)?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(Background::Image(image, color))
                }
                "patch" => {
                    c.take(TokenValue::ParenOpen)?;
                    let image = match c.tokens.next() {
                        Some(Token(TokenValue::Path(url), _)) => {
                            if c.patches.get(&url).is_none() {
                                let image = image::load_from_memory(
                                    c.loader.load(url.clone()).await.map_err(Error::Io)?.as_ref(),
                                )?;
                                c.patches.insert(url.clone(), c.cache.load_patch(image.to_rgba()));
                            }
                            Ok(c.patches[&url].clone())
                        }
                        Some(Token(_, pos)) => Err(Error::Syntax("Expected url".into(), pos)),
                        None => Err(Error::Eof),
                    }?;
                    c.take(TokenValue::Comma)?;
                    let color = parse_color(c)?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(Background::Patch(image, color))
                }
                _ => Err(Error::Syntax("Expected `image`, `patch` or `none`".into(), pos)),
            }
        }
        Token(TokenValue::Color(_), _) => Ok(Background::Color(parse_color(c)?)),
        Token(TokenValue::Path(url), _) => {
            c.tokens.next();
            if url.ends_with(".9.png") {
                if c.patches.get(&url).is_none() {
                    let image = image::load_from_memory(c.loader.load(url.clone()).await.map_err(Error::Io)?.as_ref())?;
                    c.patches.insert(url.clone(), c.cache.load_patch(image.to_rgba()));
                }
                Ok(Background::Patch(c.patches[&url].clone(), Color::white()))
            } else {
                if c.images.get(&url).is_none() {
                    let image = image::load_from_memory(c.loader.load(url.clone()).await.map_err(Error::Io)?.as_ref())?;
                    c.images.insert(url.clone(), c.cache.load_image(image.to_rgba()));
                }
                Ok(Background::Image(c.images[&url].clone(), Color::white()))
            }
        }
        Token(_, pos) => Err(Error::Syntax(
            "Expected `none`, `image(<url>, <color>)`, `patch(<url>, <color>)`, <color> or <url>".into(),
            pos,
        )),
    }
}

async fn parse_font<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Font, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Path(url), _)) => {
            if c.fonts.get(&url).is_none() {
                let font = c.cache.load_font(c.loader.load(url.as_str()).await.map_err(Error::Io)?);
                c.fonts.insert(url.clone(), font);
            }
            Ok(c.fonts[&url].clone())
        }
        Some(Token(_, pos)) => Err(Error::Syntax("Expected <url>".into(), pos)),
        None => Err(Error::Eof),
    }
}

fn parse_selector<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<Selector, Error<L::Error>> {
    match c.tokens.next().ok_or(Error::Eof)? {
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
                    let result = match c.tokens.next().ok_or(Error::Eof)? {
                        Token(TokenValue::Iden(special), pos) => match special.as_str() {
                            "odd" => Ok(Selector::NthMod(1, 2)),
                            "even" => Ok(Selector::NthMod(0, 2)),
                            _ => Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos)),
                        },
                        Token(TokenValue::Number(number), pos) => Ok(Selector::Nth(
                            number
                                .parse::<usize>()
                                .map_err(|err| Error::Syntax(format!("{}", err), pos))?,
                        )),
                        Token(_, pos) => Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos)),
                    }?;
                    c.take(TokenValue::ParenClose)?;
                    Ok(result)
                }
                "nth-last-child" => {
                    c.take(TokenValue::ParenOpen)?;
                    let result = match c.tokens.next().ok_or(Error::Eof)? {
                        Token(TokenValue::Iden(special), pos) => match special.as_str() {
                            "odd" => Ok(Selector::NthLastMod(1, 2)),
                            "even" => Ok(Selector::NthLastMod(0, 2)),
                            _ => Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos)),
                        },
                        Token(TokenValue::Number(number), pos) => Ok(Selector::NthLast(
                            number
                                .parse::<usize>()
                                .map_err(|err| Error::Syntax(format!("{}", err), pos))?,
                        )),
                        Token(_, pos) => Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos)),
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
                state => Ok(Selector::State(state.to_string())),
            }
        }
        Token(_, pos) => Err(Error::Syntax("expected `<selector`>".into(), pos)),
    }
}

fn parse_widget<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<SelectorWidget, Error<L::Error>> {
    match c.tokens.next().ok_or(Error::Eof)? {
        Token(TokenValue::Star, _) => Ok(SelectorWidget::Any),
        Token(TokenValue::Iden(widget), _) => Ok(SelectorWidget::Some(widget)),
        Token(_, pos) => Err(Error::Syntax("Expected '*' or 'identifier'".into(), pos)),
    }
}

fn parse_float<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<f32, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Number(number), pos)) => number
            .parse::<f32>()
            .map_err(|err| Error::Syntax(format!("{}", err), pos)),
        Some(Token(_, pos)) => Err(Error::Syntax("Expected <number>".into(), pos)),
        None => Err(Error::Eof),
    }
}

fn parse_usize<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<usize, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Number(number), pos)) => number
            .parse::<usize>()
            .map_err(|err| Error::Syntax(format!("{}", err), pos)),
        Some(Token(_, pos)) => Err(Error::Syntax("Expected <integer>".into(), pos)),
        None => Err(Error::Eof),
    }
}

fn parse_rectangle<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<Rectangle, Error<L::Error>> {
    let mut result = Rectangle::zero();
    match c.tokens.next() {
        Some(Token(TokenValue::ParenOpen, _)) => loop {
            match c.tokens.next() {
                Some(Token(TokenValue::Iden(field), pos)) => {
                    c.take(TokenValue::Colon)?;
                    match field.as_str() {
                        "left" => result.left = parse_float(c)?,
                        "top" => result.top = parse_float(c)?,
                        "right" => result.right = parse_float(c)?,
                        "bottom" => result.bottom = parse_float(c)?,
                        _ => Err(Error::Syntax("Expected `left`, `top`, `right` or `bottom`".into(), pos))?,
                    }
                }
                Some(Token(TokenValue::ParenClose, _)) => {
                    return Ok(result);
                }
                Some(Token(_, pos)) => {
                    return Err(Error::Syntax(
                        "Expected `left`, `top`, `right`, `bottom` or `)`".into(),
                        pos,
                    ))
                }
                None => return Err(Error::Eof),
            }
        },
        Some(Token(TokenValue::Number(number), pos)) => {
            let uniform = number
                .parse::<f32>()
                .map_err(|err| Error::Syntax(format!("{}", err), pos))?;
            Ok(Rectangle {
                left: uniform,
                top: uniform,
                right: uniform,
                bottom: uniform,
            })
        }
        Some(Token(_, pos)) => Err(Error::Syntax("Expected `(` or <number>".into(), pos)),
        None => Err(Error::Eof),
    }
}

fn parse_text_wrap<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<TextWrap, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "no-wrap" => Ok(TextWrap::NoWrap),
            "word-wrap" => Ok(TextWrap::WordWrap),
            "wrap" => Ok(TextWrap::Wrap),
            _ => Err(Error::Syntax("Expected `no-wrap`, `word-wrap` or `wrap`".into(), pos)),
        },
        Some(Token(_, pos)) => Err(Error::Syntax("Expected `no-wrap`, `word-wrap` or `wrap`".into(), pos)),
        None => Err(Error::Eof),
    }
}

fn parse_align<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Align, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "begin" | "left" | "top" => Ok(Align::Begin),
            "center" => Ok(Align::Center),
            "end" | "right" | "bottom" => Ok(Align::End),
            _ => Err(Error::Syntax("Expected `begin`, `center` or `end`".into(), pos)),
        },
        Some(Token(_, pos)) => Err(Error::Syntax("Expected `begin`, `center` or `end`".into(), pos)),
        None => Err(Error::Eof),
    }
}

fn parse_size<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Size, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Iden(ty), pos)) => match ty.to_lowercase().as_str() {
            "shrink" => Ok(Size::Shrink),
            "fill" => {
                c.take(TokenValue::ParenOpen)?;
                let size = parse_usize(c)?;
                c.take(TokenValue::ParenClose)?;
                Ok(Size::Fill(size as u32))
            }
            _ => Err(Error::Syntax(
                "Expected `shrink`, `fill(<integer>)` or <number>".into(),
                pos,
            )),
        },
        Some(Token(TokenValue::Number(num), pos)) => Ok(Size::Exact(
            num.parse::<f32>()
                .map_err(|err| Error::Syntax(format!("{}", err), pos))?,
        )),
        Some(Token(_, pos)) => Err(Error::Syntax(
            "Expected `shrink`, `fill(<integer>)` or <number>".into(),
            pos,
        )),
        None => Err(Error::Eof),
    }
}

fn parse_color<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Color, Error<L::Error>> {
    match c.tokens.next().ok_or(Error::Eof)? {
        Token(TokenValue::Color(string), pos) => {
            let int = u32::from_str_radix(string.as_str(), 16).map_err(|err| Error::Syntax(format!("{}", err), pos))?;
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
                _ => Err(Error::Syntax(
                    "Color values must match one of the following hex patterns: #rgb, #rgba, #rrggbb or #rrggbbaa"
                        .into(),
                    pos,
                )),
            }
        }
        Token(_, pos) => Err(Error::Syntax("Expected <color>".into(), pos)),
    }
}

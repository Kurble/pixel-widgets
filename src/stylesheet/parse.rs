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

    let mut rule_tree = NewRuleTree {
        selector: Selector::Widget(SelectorWidget::Any),
        rules: Vec::new(),
        children: Vec::new(),
    };
    while let Some(_) = context.tokens.peek() {
        let (selectors, rules) = parse_block(&mut context).await?;
        rule_tree.insert(selectors, rules);
    }

    result.rule_tree = rule_tree.into();

    Ok(result)
}

async fn parse_block<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<(Vec<Selector>, Vec<Rule>), Error<L::Error>> {
    let mut selectors = Vec::new();
    let mut rules = Vec::new();

    loop {
        match c.tokens.next() {
            Some(Token(TokenValue::Star, _)) => selectors.push(Selector::Widget(SelectorWidget::Any)),
            Some(Token(TokenValue::Dot, _)) => selectors.push(Selector::Class(c.take_identifier()?.0)),
            Some(Token(TokenValue::Iden(widget), _)) => selectors.push(Selector::Widget(SelectorWidget::Some(widget))),
            Some(Token(TokenValue::Gt, _)) => selectors.push(Selector::WidgetDirectChild(parse_widget(c)?)),
            Some(Token(TokenValue::Plus, _)) => selectors.push(Selector::WidgetDirectAfter(parse_widget(c)?)),
            Some(Token(TokenValue::Tilde, _)) => selectors.push(Selector::WidgetAfter(parse_widget(c)?)),
            Some(Token(TokenValue::Colon, _)) => {
                let (id, _) = c.take_identifier()?;
                match id.as_str() {
                    "nth-child-mod" => {
                        c.take(TokenValue::ParenOpen)?;
                        let numerator = parse_usize(c)?;
                        c.take(TokenValue::Comma)?;
                        let denominator = parse_usize(c)?;
                        c.take(TokenValue::ParenClose)?;
                        selectors.push(Selector::NthMod(numerator, denominator));
                    }
                    "nth-last-child-mod" => {
                        c.take(TokenValue::ParenOpen)?;
                        let numerator = parse_usize(c)?;
                        c.take(TokenValue::Comma)?;
                        let denominator = parse_usize(c)?;
                        c.take(TokenValue::ParenClose)?;
                        selectors.push(Selector::NthLastMod(numerator, denominator));
                    }
                    "nth-child" => {
                        c.take(TokenValue::ParenOpen)?;
                        match c.tokens.next().ok_or(Error::Eof)? {
                            Token(TokenValue::Iden(special), pos) => match special.as_str() {
                                "odd" => selectors.push(Selector::NthMod(1, 2)),
                                "even" => selectors.push(Selector::NthMod(0, 2)),
                                _ => return Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos)),
                            },
                            Token(TokenValue::Number(number), pos) => {
                                selectors.push(Selector::Nth(
                                    number
                                        .parse::<usize>()
                                        .map_err(|err| Error::Syntax(format!("{}", err), pos))?,
                                ));
                            }
                            Token(_, pos) => {
                                return Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos))
                            }
                        }
                        c.take(TokenValue::ParenClose)?;
                    }
                    "nth-last-child" => {
                        c.take(TokenValue::ParenOpen)?;
                        match c.tokens.next().ok_or(Error::Eof)? {
                            Token(TokenValue::Iden(special), pos) => match special.as_str() {
                                "odd" => selectors.push(Selector::NthLastMod(1, 2)),
                                "even" => selectors.push(Selector::NthLastMod(0, 2)),
                                _ => return Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos)),
                            },
                            Token(TokenValue::Number(number), pos) => {
                                selectors.push(Selector::NthLast(
                                    number
                                        .parse::<usize>()
                                        .map_err(|err| Error::Syntax(format!("{}", err), pos))?,
                                ));
                            }
                            Token(_, pos) => {
                                return Err(Error::Syntax("Expected 'odd', 'even' or <number>.".into(), pos))
                            }
                        }
                        c.take(TokenValue::ParenClose)?;
                    }
                    "first-child" => selectors.push(Selector::Nth(0)),
                    "last-child" => selectors.push(Selector::NthLast(0)),
                    state => selectors.push(Selector::State(state.to_string())),
                }
            }
            Some(Token(TokenValue::BraceOpen, _)) => {
                loop {
                    if let Some(&Token(TokenValue::BraceClose, _)) = c.tokens.peek() {
                        break;
                    } else {
                        rules.push(parse_rule(c).await?);
                    }
                }
                c.take(TokenValue::BraceClose)?;
                return Ok((selectors, rules));
            }
            Some(Token(_, pos)) => return Err(Error::Syntax("Expected <selector> or `{`".into(), pos)),
            None => return Err(Error::Eof),
        }
    }
}

async fn parse_rule<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Rule, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Iden(key), pos)) => {
            c.take(TokenValue::Colon)?;
            match key.as_str() {
                "background" => Ok(Rule::Background(parse_background(c).await?)),
                "font" => Ok(Rule::Font(parse_font(c).await?)),
                "color" => Ok(Rule::Color(parse_color(c)?)),
                "scrollbar-horizontal" => Ok(Rule::ScrollbarHorizontal(parse_background(c).await?)),
                "scrollbar-vertical" => Ok(Rule::ScrollbarVertical(parse_background(c).await?)),
                "padding" => Ok(Rule::Padding(parse_rectangle(c)?)),
                "text-size" => Ok(Rule::TextSize(parse_float(c)?)),
                "text-wrap" => Ok(Rule::TextWrap(parse_text_wrap(c)?)),
                "width" => Ok(Rule::Width(parse_size(c)?)),
                "height" => Ok(Rule::Height(parse_size(c)?)),
                "align-horizontal" => Ok(Rule::AlignHorizontal(parse_align(c)?)),
                "align-vertical" => Ok(Rule::AlignVertical(parse_align(c)?)),
                unrecognized => Err(Error::Syntax(format!("Rule '{}' not recognized", unrecognized), pos)),
            }
        }
        Some(Token(_, pos)) => Err(Error::Syntax("Expected <identifier>".into(), pos)),
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

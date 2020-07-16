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
        let next = self
            .tokens
            .next()
            .ok_or(Error::Syntax(format!("Expected '{:?}' at end of file", token)))?;
        if token == next.0 {
            Ok(next)
        } else {
            Err(Error::Syntax(format!(
                "Expected '{:?}' at {}:{}, found '{:?}'",
                token, next.1.line, next.1.col_start, next.0
            )))
        }
    }
}

pub async fn parse<L: Loader>(
    tokens: Vec<Token>,
    loader: &L,
    cache: &mut Cache,
) -> Result<Style, Error<L::Error>> {
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

    while let Some(_) = context.tokens.peek() {
        result.selectors.push(parse_selector(&mut context).await?);
    }

    Ok(result)
}

async fn parse_selector<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Selector, Error<L::Error>> {
    let mut selector = Selector {
        elements: vec![],
        classes: vec![],
        rules: vec![],
    };

    loop {
        match c.tokens.next() {
            Some(Token(TokenValue::Identifier(element), _)) => {
                selector.elements.push(element);
            }
            Some(Token(TokenValue::Class(class), _)) => {
                selector.classes.push(class);
            }
            Some(Token(TokenValue::CurlyOpen, _)) => {
                loop {
                    if let Some(&Token(TokenValue::CurlyClose, _)) = c.tokens.peek() {
                        break;
                    } else {
                        selector.rules.push(parse_rule(c).await?);
                    }
                }
                c.take(TokenValue::CurlyClose)?;
                return Ok(selector);
            }
            Some(other) => return Err(Error::Syntax(format!("Unexpected token parsing selector: {:?}", other))),
            None => return Err(Error::Syntax("unexpected end of file".into())),
        }
    }
}

async fn parse_rule<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Rule, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Identifier(key), _)) => {
            c.take(TokenValue::Colon)?;
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
        Some(Token(tok, pos)) => Err(Error::Syntax(format!(
            "{}:{}: Expected identifier at start of rule, got '{:?}'",
            pos.line, pos.col_start, tok
        ))),
        None => Err(Error::Syntax("Unexpected end of file parsing rule".into())),
    }
}

async fn parse_background<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Background, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "none" => Ok(Background::None),
            "image" => {
                c.take(TokenValue::BraceOpen)?;
                let image = if let Some(Token(TokenValue::Path(url), _)) = c.tokens.next() {
                    if c.images.get(&url).is_none() {
                        let image =
                            image::load_from_memory(c.loader.load(url.clone()).await.map_err(Error::Custom)?.as_ref())?;
                        c.images.insert(url.clone(), c.cache.load_image(image.to_rgba()));
                    }
                    Ok(c.images[&url].clone())
                } else {
                    Err(Error::Syntax("Expected url".into()))
                }?;
                c.take(TokenValue::Comma)?;
                let color = parse_color::<L>(c.tokens.next())?;
                c.take(TokenValue::BraceClose)?;
                Ok(Background::Image(image, color))
            }
            "patch" => {
                c.take(TokenValue::BraceOpen)?;
                let image = if let Some(Token(TokenValue::Path(url), _)) = c.tokens.next() {
                    if c.patches.get(&url).is_none() {
                        let image =
                            image::load_from_memory(c.loader.load(url.clone()).await.map_err(Error::Custom)?.as_ref())?;
                        c.patches.insert(url.clone(), c.cache.load_patch(image.to_rgba()));
                    }
                    Ok(c.patches[&url].clone())
                } else {
                    Err(Error::Syntax("Expected url".into()))
                }?;
                c.take(TokenValue::Comma)?;
                let color = parse_color::<L>(c.tokens.next())?;
                c.take(TokenValue::BraceClose)?;
                Ok(Background::Patch(image, color))
            }
            other => Err(Error::Syntax(format!("{} is not a background type", other))),
        },
        Some(Token(TokenValue::Color(color), pos)) => Ok(Background::Color(parse_color::<L>(Some(Token(
            TokenValue::Color(color),
            pos,
        )))?)),
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing background".into())),
    }
}

async fn parse_font<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<'_, I, L>,
) -> Result<Font, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Path(url), _)) => {
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

fn parse_float<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<f32, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Number(number), _)) => number
            .parse::<f32>()
            .ok()
            .ok_or(Error::Syntax("unable to parse float".into())),
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_rectangle<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<Rectangle, Error<L::Error>> {
    let mut result = Rectangle::zero();
    c.take(TokenValue::BraceOpen)?;
    loop {
        match c.tokens.next() {
            Some(Token(TokenValue::Identifier(field), _)) => {
                c.take(TokenValue::Colon)?;
                match field.as_str() {
                    "left" => result.left = parse_float(c)?,
                    "top" => result.top = parse_float(c)?,
                    "right" => result.right = parse_float(c)?,
                    "bottom" => result.bottom = parse_float(c)?,
                    other => Err(Error::Syntax(format!("unknown field '{}' for rectangle", other)))?,
                }
            }
            Some(Token(TokenValue::BraceClose, _)) => {
                return Ok(result);
            }
            Some(other) => return Err(Error::Syntax(format!("Unexpected token {:?}", other))),
            None => return Err(Error::Syntax("Unexpected end of file while parsing rectangle".into())),
        }
    }
}

fn parse_text_wrap<I: Iterator<Item = Token>, L: Loader>(
    c: &mut LoadContext<I, L>,
) -> Result<TextWrap, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
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

fn parse_align<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Align, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "begin" | "left" | "top" => Ok(Align::Begin),
            "center" => Ok(Align::Center),
            "end" | "right" | "bottom" => Ok(Align::End),
            other => Err(Error::Syntax(format!("{} is not one of begin, center or end", other))),
        },
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_size<I: Iterator<Item = Token>, L: Loader>(c: &mut LoadContext<I, L>) -> Result<Size, Error<L::Error>> {
    match c.tokens.next() {
        Some(Token(TokenValue::Identifier(ty), _)) => match ty.to_lowercase().as_str() {
            "shrink" => Ok(Size::Shrink),
            "fill" => {
                c.take(TokenValue::BraceOpen)?;
                let size = parse_float(c)?;
                c.take(TokenValue::BraceClose)?;
                Ok(Size::Fill(size as u32))
            }
            other => Err(Error::Syntax(format!("{} is not one of begin, center or end", other))),
        },
        Some(Token(TokenValue::Number(num), _)) => Ok(Size::Exact(
            num.parse::<f32>()
                .ok()
                .ok_or(Error::Syntax("unable to parse exact size".into()))?,
        )),
        Some(other) => Err(Error::Syntax(format!("Unexpected token {:?}", other))),
        None => Err(Error::Syntax("Unexpected end of file while parsing font".into())),
    }
}

fn parse_color<L: Loader>(token: Option<Token>) -> Result<Color, Error<L::Error>> {
    if let Some(Token(TokenValue::Color(string), _)) = token {
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

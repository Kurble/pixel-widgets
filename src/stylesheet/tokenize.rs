use super::Error;

const URL_CHARACTERS: &'static str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~:/?#[]@!$&'()*+,;%=";
const NUMBER_CHARACTERS: &'static str = "0123456789.";

#[derive(Debug, Clone, Copy)]
pub struct TokenPos {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenValue {
    Identifier(String),
    Class(String),
    Path(String),
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
pub struct Token(pub TokenValue, pub TokenPos);

pub fn tokenize<E: std::error::Error>(text: String) -> Result<Vec<Token>, Error<E>> {
    let mut line = 1;
    let mut col = 0;

    let mut current: Option<Token> = None;
    let mut tokens = Vec::new();

    for chr in text.chars() {
        col += 1;

        let extended = current.as_mut().map(|current| current.extend(chr)).unwrap_or(false);

        if !extended {
            tokens.extend(current.take());

            let pos = TokenPos {
                col_start: col,
                col_end: col,
                line,
            };

            current = match chr {
                '\n' => {
                    line += 1;
                    col = 0;
                    None
                }
                chr if chr.is_whitespace() => None,
                chr if chr.is_alphabetic() || chr == '_' || chr == '-' => {
                    Some(Token(TokenValue::Identifier(chr.to_string()), pos))
                }
                chr if chr.is_numeric() => Some(Token(TokenValue::Number(chr.to_string()), pos)),
                '.' => Some(Token(TokenValue::Class(String::new()), pos)),
                '@' => Some(Token(TokenValue::Path(String::new()), pos)),
                '#' => Some(Token(TokenValue::Color(String::new()), pos)),
                '(' => Some(Token(TokenValue::BraceOpen, pos)),
                ')' => Some(Token(TokenValue::BraceClose, pos)),
                '{' => Some(Token(TokenValue::CurlyOpen, pos)),
                '}' => Some(Token(TokenValue::CurlyClose, pos)),
                ':' => Some(Token(TokenValue::Colon, pos)),
                ';' => Some(Token(TokenValue::Semi, pos)),
                ',' => Some(Token(TokenValue::Comma, pos)),
                chr => {
                    return Err(Error::Syntax(format!(
                        "Unexpected character '{}' at line {} col {}",
                        chr, line, col
                    )))
                }
            };
        }
    }

    tokens.extend(current);

    Ok(tokens)
}

impl Token {
    fn extend(&mut self, ch: char) -> bool {
        match self {
            Token(TokenValue::Identifier(ref mut s), ref mut pos)
            | Token(TokenValue::Class(ref mut s), ref mut pos) => {
                if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                    pos.col_end += 1;
                    s.push(ch);
                    true
                } else {
                    false
                }
            }
            Token(TokenValue::Path(ref mut p), ref mut pos) => {
                if URL_CHARACTERS.chars().find(|&c| c == ch).is_some() {
                    pos.col_end += 1;
                    p.push(ch);
                    true
                } else {
                    false
                }
            }
            Token(TokenValue::Number(ref mut n), ref mut pos) => {
                if NUMBER_CHARACTERS.chars().find(|&c| c == ch).is_some() {
                    pos.col_end += 1;
                    n.push(ch);
                    true
                } else {
                    false
                }
            }
            Token(TokenValue::Color(ref mut c), ref mut pos) => {
                if ch.is_ascii_hexdigit() {
                    pos.col_end += 1;
                    c.push(ch);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

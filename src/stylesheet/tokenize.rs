use super::Error;

const URL_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~:/?#[]@!$&'()*+,;%=";
const NUMBER_CHARACTERS: &str = "0123456789.";

#[derive(Debug, Clone, Copy)]
pub struct TokenPos {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValue {
    Iden(String),
    Color(String),
    Path(String),
    Number(String),
    ParenOpen,
    ParenClose,
    BraceOpen,
    BraceClose,
    Colon,
    Semi,
    Dot,
    Comma,
    Gt,
    Plus,
    Tilde,
    Star,
}

#[derive(Debug, Clone)]
pub struct Token(pub TokenValue, pub TokenPos);

pub fn tokenize(text: String) -> Result<Vec<Token>, Error> {
    let mut line = 1;
    let mut col = 0;

    let mut current: Option<Token> = None;
    let mut tokens = Vec::new();

    for chr in text.chars() {
        col += 1;

        let extended = current
            .as_mut()
            .map(|current| current.extend(chr))
            .unwrap_or(ExtendResult::NotAccepted);
        match extended {
            ExtendResult::Accepted => (),
            ExtendResult::Finished => {
                tokens.extend(current.take());
            }
            ExtendResult::NotAccepted => {
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
                        Some(Token(TokenValue::Iden(chr.to_string()), pos))
                    }
                    chr if chr.is_numeric() => Some(Token(TokenValue::Number(chr.to_string()), pos)),
                    '#' => Some(Token(TokenValue::Color(String::new()), pos)),
                    '"' => Some(Token(TokenValue::Path(String::new()), pos)),
                    '(' => Some(Token(TokenValue::ParenOpen, pos)),
                    ')' => Some(Token(TokenValue::ParenClose, pos)),
                    '{' => Some(Token(TokenValue::BraceOpen, pos)),
                    '}' => Some(Token(TokenValue::BraceClose, pos)),
                    ':' => Some(Token(TokenValue::Colon, pos)),
                    ';' => Some(Token(TokenValue::Semi, pos)),
                    '.' => Some(Token(TokenValue::Dot, pos)),
                    ',' => Some(Token(TokenValue::Comma, pos)),
                    '>' => Some(Token(TokenValue::Gt, pos)),
                    '+' => Some(Token(TokenValue::Plus, pos)),
                    '~' => Some(Token(TokenValue::Tilde, pos)),
                    '*' => Some(Token(TokenValue::Star, pos)),
                    chr => {
                        return Err(Error::Syntax(format!("Unexpected character '{}'", chr), pos));
                    }
                };
            }
        }
    }

    tokens.extend(current);

    Ok(tokens)
}

enum ExtendResult {
    Accepted,
    Finished,
    NotAccepted,
}

impl Token {
    fn extend(&mut self, ch: char) -> ExtendResult {
        match self {
            Token(TokenValue::Iden(ref mut s), ref mut pos) => {
                if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                    pos.col_end += 1;
                    s.push(ch);
                    ExtendResult::Accepted
                } else {
                    ExtendResult::NotAccepted
                }
            }
            Token(TokenValue::Color(ref mut p), ref mut pos) => {
                if ch.is_ascii_hexdigit() {
                    pos.col_end += 1;
                    p.push(ch);
                    ExtendResult::Accepted
                } else {
                    ExtendResult::NotAccepted
                }
            }
            Token(TokenValue::Path(ref mut p), ref mut pos) => {
                if URL_CHARACTERS.chars().any(|c| c == ch) {
                    pos.col_end += 1;
                    p.push(ch);
                    ExtendResult::Accepted
                } else if ch == '"' {
                    ExtendResult::Finished
                } else {
                    ExtendResult::NotAccepted
                }
            }
            Token(TokenValue::Number(ref mut n), ref mut pos) => {
                if NUMBER_CHARACTERS.chars().any(|c| c == ch) {
                    pos.col_end += 1;
                    n.push(ch);
                    ExtendResult::Accepted
                } else {
                    ExtendResult::NotAccepted
                }
            }
            _ => ExtendResult::NotAccepted,
        }
    }
}

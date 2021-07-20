use crate::command::Command;
use std::error::Error;

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
pub enum ParseError {
    UnknownToken(usize),
    UnknownCommand, // TODO: Use the unknown command in the error message
    Other,
}

impl Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownToken(len) => write!(f, "Unexpected input at {}", len),
            ParseError::UnknownCommand => write!(f, "No such command"),
            ParseError::Other => write!(f, "Something happened"),
        }
    }
}

pub struct Token {
    pub kind: TokenKind,
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenKind {
    WS,
    CRLF,
    Word,
}

#[derive(Debug, Clone)]
pub struct Tokenizer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            data: input,
            pos: 0,
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        let input = &self.data[self.pos..];
        if !input.is_empty() {
            if let Some(len) = whitespace(input) {
                let token = Token {
                    kind: TokenKind::WS,
                    start_pos: self.pos,
                    end_pos: self.pos + len,
                };
                self.pos += len;
                return Some(Ok(token));
            }
            if let Some(len) = crlf(input) {
                let token = Token {
                    kind: TokenKind::CRLF,
                    start_pos: self.pos,
                    end_pos: self.pos + len,
                };
                self.pos += len;
                return Some(Ok(token));
            }
            if let Some(len) = word(input) {
                let token = Token {
                    kind: TokenKind::Word,
                    start_pos: self.pos,
                    end_pos: self.pos + len,
                };
                self.pos += len;
                return Some(Ok(token));
            }
            Some(Err(ParseError::UnknownToken(self.pos)))
        } else {
            None
        }
    }
}

pub fn parse(input: &[u8]) -> Result<Command> {
    let mut tokenizer = Tokenizer::new(input);

    let token = tokenizer.next().ok_or(ParseError::UnknownCommand)??;
    let name = if token.kind == TokenKind::Word {
        &input[token.start_pos..token.end_pos]
    } else {
        return Err(ParseError::Other);
    };

    let args: Vec<Token> = tokenizer
        .by_ref()
        .take_while(|r| r.as_ref().map(|t| t.kind != TokenKind::CRLF).is_ok())
        .map(|r| r.unwrap())
        .filter(|t| t.kind == TokenKind::Word)
        .collect();

    let get_arg = |i| {
        let token: &Token = &args[i];
        let s = &input[token.start_pos..token.end_pos];
        String::from_utf8(s.to_vec()).unwrap()
    };

    match name {
        b"get" => Ok(Command::Get(get_arg(0))),
        b"set" => Ok(Command::Set(get_arg(0), get_arg(1))),
        b"remove" => Ok(Command::Remove(get_arg(0))),
        _ => Err(ParseError::Other),
    }
}

fn whitespace(input: &[u8]) -> Option<usize> {
    let count = input.iter().take_while(|&&b| b == b' ').count();
    if count > 0 {
        Some(count)
    } else {
        None
    }
}

fn crlf(input: &[u8]) -> Option<usize> {
    if input.len() >= 2 && &input[0..2] == b"\r\n" {
        Some(2)
    } else {
        None
    }
}

fn word(input: &[u8]) -> Option<usize> {
    let count = input
        .iter()
        .take_while(|&&b| is_word_char_upper(b) || is_word_char_lower(b))
        .count();
    if count > 0 {
        Some(count)
    } else {
        None
    }
}

fn is_word_char_upper(byte: u8) -> bool {
    (65..=90).contains(&byte)
}

fn is_word_char_lower(byte: u8) -> bool {
    (97..=122).contains(&byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_logger() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .is_test(true)
            .try_init();
    }

    #[test]
    fn tokenizer_happy_path() {
        let input = b"hello world\r\n";
        let mut tokens = Tokenizer::new(input);
        let mut next = || tokens.next().unwrap().unwrap();

        assert!(matches!(next().kind, TokenKind::Word));
        assert!(matches!(next().kind, TokenKind::WS));
        assert!(matches!(next().kind, TokenKind::Word));
        assert!(matches!(next().kind, TokenKind::CRLF));
        assert!(matches!(tokens.next(), None));
    }

    #[test]
    fn tokenizer_take_while() {
        let input = b"hello world\r\n";
        let mut tokenizer = Tokenizer::new(input);
        let tokens: Vec<Token> = tokenizer
            .by_ref()
            .take_while(|r| r.as_ref().map(|t| t.kind != TokenKind::CRLF).is_ok())
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(4, tokens.len());
        assert_eq!(13, tokenizer.pos);
    }

    #[test]
    #[ignore]
    fn tokenizer_bad_input() {
        todo!();
    }

    #[test]
    fn parse_get() {
        init_logger();
        log::debug!("Hello");
        let input = b"get drink\r\n";
        let cmd = parse(input).unwrap();
        assert!(matches!(cmd, Command::Get(_)));
    }

    #[test]
    fn parse_set() {
        init_logger();
        log::debug!("Hello");
        let input = b"set drink whisky\r\n";
        let cmd = parse(input).unwrap();
        assert!(matches!(cmd, Command::Set(..)));
    }

    #[test]
    fn parse_remove() {
        init_logger();
        log::debug!("Hello");
        let input = b"remove drink\r\n";
        let cmd = parse(input).unwrap();
        assert!(matches!(cmd, Command::Remove(_)));
    }
}

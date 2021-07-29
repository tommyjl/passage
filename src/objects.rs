use std::fmt::Display;
use std::io::prelude::*;
use std::io::Cursor;
use std::string::FromUtf8Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Incomplete,
    InvalidInput,
    Io(std::io::Error),
    Utf8(FromUtf8Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Incomplete => write!(f, "Incomplete data"),
            Error::InvalidInput => write!(f, "Could not identify a valid Object type"),
            Error::Io(err) => write!(f, "{}", err),
            Error::Utf8(err) => write!(f, "{}", err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Error::Utf8(err)
    }
}

pub enum Object {
    Array(Vec<Object>),
    SimpleString(String),
    Error(String),
    Integer(i64),
}

#[allow(dead_code)]
pub fn parse(input: &mut Cursor<&[u8]>) -> Result<Object> {
    if input.get_ref().is_empty() {
        return Err(Error::Incomplete);
    }

    match get_u8(input).unwrap() {
        b'+' => Ok(Object::SimpleString(read_simple(input)?)),
        b'-' => Ok(Object::Error(read_simple(input)?)),
        b':' => Ok(Object::Integer(read_integer(input)?)),
        b'*' => Ok(Object::Array(read_array(input)?)),
        _ => Err(Error::InvalidInput),
    }
}

fn get_u8(input: &mut Cursor<&[u8]>) -> Result<u8> {
    let mut buf = [0; 1];
    input.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn peek(input: &mut Cursor<&[u8]>) -> Result<u8> {
    let mut buf = [0; 1];
    input.read_exact(&mut buf)?;
    input.set_position(input.position() - 1);
    Ok(buf[0])
}

fn advance(input: &mut Cursor<&[u8]>) {
    input.set_position(input.position() + 1);
}

fn read_array(input: &mut Cursor<&[u8]>) -> Result<Vec<Object>> {
    let size = read_integer(input)? as usize;
    read_crlf(input)?;

    let mut ret = Vec::with_capacity(size);
    for _ in 0..size {
        let cmd = parse(input)?;
        ret.push(cmd);
    }

    Ok(ret)
}

fn read_integer(input: &mut Cursor<&[u8]>) -> Result<i64> {
    let sign = if peek(input)? == b'-' {
        advance(input);
        -1
    } else {
        1
    };

    let start = input.position() as usize;
    let mut end = start;

    while is_digit(peek(input)?) {
        advance(input);
        end += 1;
    }

    if start == end {
        Err(Error::InvalidInput)
    } else {
        let int = input.get_ref()[start..end]
            .iter()
            .map(|b| b & 0xF)
            .fold(0i64, |acc, b| acc * 10 + (b & 0xF) as i64);
        Ok(sign * int)
    }
}

fn read_crlf(input: &mut Cursor<&[u8]>) -> Result<()> {
    let orig_pos = input.position();
    if (input.get_ref().len() - input.position() as usize) < 2 {
        return Err(Error::Incomplete);
    }

    let next_2 = (get_u8(input)?, get_u8(input)?);
    if let (b'\r', b'\n') = next_2 {
        Ok(())
    } else {
        input.set_position(orig_pos);
        Err(Error::InvalidInput)
    }
}

fn has_remaining(input: &Cursor<&[u8]>) -> bool {
    let size = input.get_ref().len();
    let remaining = size - input.position() as usize;
    remaining > 0
}

// TODO: UTF-8
fn read_simple(input: &mut Cursor<&[u8]>) -> Result<String> {
    let start = input.position() as usize;
    while has_remaining(input) && is_simple_string_char(peek(input)?) {
        advance(input);
    }
    let end = input.position() as usize;
    read_crlf(input)?;

    let s = String::from_utf8(input.get_ref()[start..end].into())?;
    Ok(s)
}

fn is_simple_string_char(input: u8) -> bool {
    (0x20..0x7F).contains(&input)
}

fn is_digit(input: u8) -> bool {
    (0x30..0x3A).contains(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_string_ok() {
        let bytes: &[u8] = b"+Hello world\r\n";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor).unwrap();
        assert!(matches!(o, Object::SimpleString(_)));
        if let Object::SimpleString(s) = o {
            assert_eq!(s, "Hello world".to_string());
        }
    }

    #[test]
    fn parse_simple_string_incomplete() {
        let bytes: &[u8] = b"+Hello world";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor);
        assert!(matches!(o, Err(Error::Incomplete)));

        let bytes: &[u8] = b"+Hello world\r";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor);
        assert!(matches!(o, Err(Error::Incomplete)));
    }

    #[test]
    fn parse_error_ok() {
        let bytes: &[u8] = b"-Error: Something went wrong\r\n";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor).unwrap();
        assert!(matches!(o, Object::Error(_)));
        if let Object::Error(s) = o {
            assert_eq!(s, "Error: Something went wrong".to_string());
        }
    }

    #[test]
    fn parse_error_incomplete() {
        let bytes: &[u8] = b"-Hello world";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor);
        assert!(matches!(o, Err(Error::Incomplete)));

        let bytes: &[u8] = b"-Hello world\r";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor);
        assert!(matches!(o, Err(Error::Incomplete)));
    }

    #[test]
    fn parse_integer_positive_ok() {
        let bytes: &[u8] = b":1234567890\r\n";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor).unwrap();
        assert!(matches!(o, Object::Integer(_)));
        if let Object::Integer(int) = o {
            assert_eq!(int, 1234567890);
        }
    }

    #[test]
    fn parse_integer_negative_ok() {
        let bytes: &[u8] = b":-1234567890\r\n";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor).unwrap();
        assert!(matches!(o, Object::Integer(_)));
        if let Object::Integer(int) = o {
            assert_eq!(int, -1234567890);
        }
    }

    #[test]
    fn parse_array_ok() {
        let bytes: &[u8] = b"*2\r\n+Hello world\r\n+Goodbye world\r\n";
        let mut cursor = Cursor::new(bytes);
        let o = parse(&mut cursor).unwrap();
        assert!(matches!(o, Object::Array(_)));
        if let Object::Array(a) = o {
            assert_eq!(a.len(), 2);
            assert!(matches!(a[0], Object::SimpleString(_)));
            assert!(matches!(a[1], Object::SimpleString(_)));
        }
    }
}

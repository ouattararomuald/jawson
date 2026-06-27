use crate::errors::ParseError;
use std::io::{Bytes, Read};
use std::iter::Peekable;

const BEGIN_ARRAY: u8 = b'[';
const BEGIN_OBJECT: u8 = b'{';
const END_ARRAY: u8 = b']';
const END_OBJECT: u8 = b'}';
const NAME_SEPARATOR: u8 = b':';
const VALUE_SEPARATOR: u8 = b',';

const DOUBLE_QUOTE: u8 = b'"';
const BOOLEAN_TRUE: u8 = b't';
const BOOLEAN_FALSE: u8 = b'f';
const NULL: u8 = b'n';

// Numbers
const ZERO: u8 = b'0';
const NINE: u8 = b'9';
const MINUS: u8 = b'-';
const PLUS: u8 = b'+';
const EXPONENT: u8 = b'e';
const EXPONENT_UPPER: u8 = b'E';
const DECIMAL_POINT: u8 = b'.';

const SPACE: u8 = b' ';
const TAB: u8 = b'\t';
const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

const IS_WHITESPACE: [bool; 256] = {
    let mut table = [false; 256];
    table[SPACE as usize] = true;
    table[TAB as usize] = true;
    table[LINE_FEED as usize] = true;
    table[CARRIAGE_RETURN as usize] = true;
    table
};

#[derive(Debug, PartialEq)]
pub(crate) enum ValueKind {
    BeginArray,
    BeginObject,
    EndArray,
    EndObject,
    NameSeparator,
    ValueSeparator,
    Null,
    Bool,
    Number,
    String,
}

pub(crate) fn is_whitespace(byte: u8) -> bool {
    IS_WHITESPACE[byte as usize]
}

pub(crate) fn is_number_char(byte: u8) -> bool {
    matches!(
        byte,
        ZERO..=NINE | MINUS | PLUS | DECIMAL_POINT | EXPONENT | EXPONENT_UPPER
    )
}

pub(crate) fn is_float_number(bytes: &[u8]) -> bool {
    bytes.contains(&DECIMAL_POINT) || bytes.contains(&EXPONENT) || bytes.contains(&EXPONENT_UPPER)
}

pub(crate) fn is_double_quote(byte: u8) -> bool {
    byte == DOUBLE_QUOTE
}

pub(crate) fn convert_to_bool(byte: u8) -> bool {
    if byte == BOOLEAN_TRUE {
        return true;
    }

    false
}

pub(crate) fn parse_value_kind(byte: u8) -> Result<ValueKind, ParseError> {
    match byte {
        BEGIN_ARRAY => Ok(ValueKind::BeginArray),
        BEGIN_OBJECT => Ok(ValueKind::BeginObject),
        END_ARRAY => Ok(ValueKind::EndArray),
        END_OBJECT => Ok(ValueKind::EndObject),
        NAME_SEPARATOR => Ok(ValueKind::NameSeparator),
        VALUE_SEPARATOR => Ok(ValueKind::ValueSeparator),
        DOUBLE_QUOTE => Ok(ValueKind::String),
        BOOLEAN_FALSE | BOOLEAN_TRUE => Ok(ValueKind::Bool),
        NULL => Ok(ValueKind::Null),
        ZERO..=NINE | MINUS => Ok(ValueKind::Number),
        _ => Err(ParseError::UnexpectedByte(byte)),
    }
}

pub(crate) fn read_while<R, F>(iter_bytes: &mut Peekable<Bytes<R>>, mut condition: F) -> Vec<u8>
where
    R: Read,
    F: FnMut(u8) -> bool,
{
    let mut bytes_collection = Vec::new();

    loop {
        let byte = iter_bytes.peek();
        if let Some(Ok(b)) = byte {
            if condition(*b) {
                bytes_collection.push(iter_bytes.next().unwrap().unwrap());
            } else {
                break;
            }
        } else {
            break;
        }
    }

    bytes_collection
}

pub(crate) fn read_until<R, F>(
    iter_bytes: &mut Peekable<Bytes<R>>,
    mut condition: F,
) -> Result<Vec<u8>, ParseError>
where
    R: Read,
    F: FnMut(u8) -> bool,
{
    let mut bytes_collection: Vec<u8> = Vec::new();

    loop {
        let byte = iter_bytes.next();
        match byte {
            None | Some(Err(_)) => return Err(ParseError::UnexpectedEof),
            Some(Ok(b'\\')) => {
                let e = match iter_bytes.next() {
                    Some(Ok(b)) => b,
                    _ => return Err(ParseError::UnexpectedEof),
                };
                match e {
                    b'\\' => bytes_collection.push(b'\\'),
                    b'"' => bytes_collection.push(b'"'),
                    b'/' => bytes_collection.push(b'/'),
                    b'b' => bytes_collection.push(0x08),
                    b'f' => bytes_collection.push(0x0C),
                    b'n' => bytes_collection.push(b'\n'),
                    b'r' => bytes_collection.push(b'\r'),
                    b't' => bytes_collection.push(b'\t'),
                    b'u' => {
                        let first = read_hex4(iter_bytes)?;

                        let code_point = if (0xD800..=0xDBFF).contains(&first) {
                            // High surrogate: must be followed by `\u` + a low surrogate.
                            match (iter_bytes.next(), iter_bytes.next()) {
                                (Some(Ok(b'\\')), Some(Ok(b'u'))) => {}
                                (None, _) | (_, None) => return Err(ParseError::UnexpectedEof),
                                _ => return Err(ParseError::InvalidUnicode),
                            }
                            let second = read_hex4(iter_bytes)?;
                            if !(0xDC00..=0xDFFF).contains(&second) {
                                return Err(ParseError::InvalidUnicode);
                            }
                            // Combine the pair into the real (astral) code point.
                            0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                        } else if (0xDC00..=0xDFFF).contains(&first) {
                            return Err(ParseError::InvalidUnicode); // lone low surrogate
                        } else {
                            first // ordinary BMP code point
                        };

                        let ch = char::from_u32(code_point).ok_or(ParseError::InvalidUnicode)?;

                        let mut buf = [0u8; 4];
                        let encoded = ch.encode_utf8(&mut buf);
                        bytes_collection.extend_from_slice(encoded.as_bytes());
                    }
                    _ => return Err(ParseError::InvalidEscape(e)),
                }
            }

            // Unescaped terminator (the closing quote).
            Some(Ok(b)) if condition(b) => break,

            // Ordinary byte.
            Some(Ok(b)) => bytes_collection.push(b),
        }
    }

    Ok(bytes_collection)
}

/// Read exactly 4 hex digits (the XXXX of a \uXXXX escape) into a code unit.
/// Assumes the leading `\u` has already been consumed.
fn read_hex4<R: Read>(iter_bytes: &mut Peekable<Bytes<R>>) -> Result<u32, ParseError> {
    let mut hex = [0u8; 4];
    for slot in hex.iter_mut() {
        *slot = match iter_bytes.next() {
            Some(Ok(b)) => b,
            _ => return Err(ParseError::UnexpectedEof),
        };
    }
    let hex_str = std::str::from_utf8(&hex).map_err(|_| ParseError::InvalidUnicode)?;
    u32::from_str_radix(hex_str, 16).map_err(|_| ParseError::InvalidUnicode)
}

/// Consume `rest` and verify each byte matches (for the tails of true/false/null).
pub(crate) fn expect_rest<R: Read>(
    iter_bytes: &mut Peekable<Bytes<R>>,
    rest: &[u8],
) -> Result<(), ParseError> {
    for &expected in rest {
        match iter_bytes.next() {
            Some(Ok(b)) if b == expected => {}
            Some(Ok(b)) => return Err(ParseError::UnexpectedByte(b)),
            _ => return Err(ParseError::UnexpectedEof),
        }
    }
    Ok(())
}

/// Convert collected bytes into a String, mapping UTF-8 failure to a ParseError.
pub(crate) fn bytes_to_string(bytes: Vec<u8>) -> Result<String, ParseError> {
    String::from_utf8(bytes).map_err(|e| ParseError::InvalidUtf8(e.utf8_error()))
}

pub(crate) fn convert_to_float(bytes: Vec<u8>) -> Result<f64, ParseError> {
    Ok(std::str::from_utf8(&bytes)?.parse()?)
}

pub(crate) fn convert_to_int(bytes: Vec<u8>) -> Result<i64, ParseError> {
    Ok(std::str::from_utf8(&bytes)?.parse()?)
}

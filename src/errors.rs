use thiserror::Error;

/// An error returned by [`parse`](crate::parse) when the input is not
/// valid JSON.
#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    /// The input ended while a value, string, or structure was still open.
    #[error("unexpected end of input")]
    UnexpectedEof,

    /// A byte was found where it cannot start (or end) a JSON value, for
    /// example a stray `]` or a character that begins no known value.
    #[error("unexpected byte: {0:#x}")]
    UnexpectedByte(u8),

    /// A backslash in a string was followed by a byte that is not a valid
    /// JSON escape character.
    #[error("invalid escape sequence: \\{0:#x}")]
    InvalidEscape(u8),

    /// A `\u` escape was malformed: bad hex digits, an invalid code point, or
    /// a broken or lone surrogate.
    #[error("invalid unicode escape")]
    InvalidUnicode,

    /// The digits of a number could not be parsed as an `i64`.
    #[error("invalid integer")]
    InvalidInt(#[from] std::num::ParseIntError),

    /// The digits of a number could not be parsed as an `f64`.
    #[error("invalid float")]
    InvalidFloat(#[from] std::num::ParseFloatError),

    /// Collected bytes were not valid UTF-8.
    #[error("invalid UTF-8")]
    InvalidUtf8(#[from] std::str::Utf8Error),
}

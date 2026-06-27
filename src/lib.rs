#![warn(missing_docs)]

//! A small JSON parser.
//!
//! The entry point is [`parse`], which turns a string slice into
//! a [`JsonValue`] tree or a [`ParseError`].
//!
//! ```
//! use jawson::parse;
//! use jawson::JsonValue;
//!
//! assert_eq!(parse("null").unwrap(), JsonValue::Null);
//! ```

mod errors;
mod json_value;
mod parser;
mod tokens;

pub use crate::errors::ParseError;
pub use crate::json_value::JsonValue;
pub use crate::parser::parse;

use std::collections::HashMap;

/// A parsed JSON value.
///
/// This is the tree produced by [`parse`](crate::parse). Each variant
/// maps to one of the JSON value types. Numbers are split into [`Int`] and
/// [`Float`] depending on whether the source contained a fraction or exponent.
///
/// [`Int`]: JsonValue::Int
/// [`Float`]: JsonValue::Float
#[derive(Debug, PartialEq)]
pub enum JsonValue {
    /// The JSON literal `null`.
    Null,
    /// A JSON boolean (`true` or `false`).
    Bool(bool),
    /// A JSON number with no fraction or exponent, parsed as an `i64`.
    Int(i64),
    /// A JSON number with a fraction or exponent, parsed as an `f64`.
    Float(f64),
    /// A JSON string, with escape sequences already decoded.
    String(String),
    /// A JSON array, in source order.
    Array(Vec<JsonValue>),
    /// A JSON object, keyed by member name.
    Object(HashMap<String, JsonValue>),
}

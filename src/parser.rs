use crate::errors::ParseError;
use crate::json_value::JsonValue;
use crate::tokens::{
    ValueKind, bytes_to_string, convert_to_bool, convert_to_float, convert_to_int, expect_rest,
    is_double_quote, is_float_number, is_number_char, is_whitespace, parse_value_kind, read_until,
    read_while,
};
use std::collections::HashMap;
use std::io::{BufReader, Bytes, Read};
use std::iter::Peekable;

/// Parse a JSON document from a string slice into a [`JsonValue`] tree.
///
/// The whole input is expected to be a single JSON value. Returns a
/// [`ParseError`] when the input is not valid JSON (an unexpected byte,
/// an unterminated string or structure, an invalid escape, and so on).
///
/// # Examples
///
/// ```
/// use jawson::parse;
/// use jawson::JsonValue;
///
/// let value = parse("[1, 2, 3]").unwrap();
/// assert_eq!(
///     value,
///     JsonValue::Array(vec![
///         JsonValue::Int(1),
///         JsonValue::Int(2),
///         JsonValue::Int(3),
///     ]),
/// );
/// ```
pub fn parse(input: &str) -> Result<JsonValue, ParseError> {
    let bytes = input.as_bytes();
    let reader = BufReader::new(bytes);
    let mut iter_bytes = reader.bytes().peekable();

    parse_nested(&mut iter_bytes, None)
}

fn parse_nested(
    iter_bytes: &mut Peekable<Bytes<BufReader<&[u8]>>>,
    value_kind: Option<ValueKind>,
) -> Result<JsonValue, ParseError> {
    let mut array: Option<Vec<JsonValue>> = None;
    let mut object: Option<HashMap<String, JsonValue>> = None;
    let mut bytes_buff: Vec<u8> = Vec::new();

    if let Some(current_value_kind) = value_kind {
        if current_value_kind == ValueKind::BeginArray {
            array = Some(Vec::new());
        } else if current_value_kind == ValueKind::BeginObject {
            object = Some(HashMap::new());
        }
    }

    read_while(iter_bytes, is_whitespace);

    while let Some(Ok(b)) = iter_bytes.next() {
        if is_whitespace(b) {
            continue;
        }

        let value_kind = parse_value_kind(b)?;
        match value_kind {
            ValueKind::BeginArray | ValueKind::BeginObject => {
                let json_value = parse_nested(iter_bytes, Some(value_kind))?;
                if let Some(ref mut a) = array {
                    a.push(json_value);
                } else {
                    return Ok(json_value);
                }
            }
            ValueKind::EndArray => {
                let a = array.ok_or(ParseError::UnexpectedByte(b']'))?;
                return Ok(JsonValue::Array(a));
            }
            ValueKind::EndObject => {
                let obj = object.ok_or(ParseError::UnexpectedByte(b'}'))?;
                return Ok(JsonValue::Object(obj));
            }
            ValueKind::NameSeparator => {
                if let Some(ref mut map) = object {
                    let key = bytes_to_string(std::mem::take(&mut bytes_buff))?;
                    map.insert(key, parse_nested(iter_bytes, None)?);
                }
            }
            ValueKind::ValueSeparator => {
                if let Some(ref mut a) = array {
                    let json_value = parse_nested(iter_bytes, None)?;
                    a.push(json_value);
                }
            }
            ValueKind::Null => {
                expect_rest(iter_bytes, b"ull")?; // validates n-u-l-l

                if let Some(ref mut a) = array {
                    a.push(JsonValue::Null);
                } else {
                    return Ok(JsonValue::Null);
                }
            }
            ValueKind::Bool => {
                let boolean = convert_to_bool(b);
                let rest: &[u8] = if boolean { b"rue" } else { b"alse" };
                expect_rest(iter_bytes, rest)?; // validates the keyword tail

                let json_value = JsonValue::Bool(boolean);
                if let Some(ref mut a) = array {
                    a.push(json_value);
                } else {
                    return Ok(json_value);
                }
            }
            ValueKind::Number => {
                let mut bytes = read_while(iter_bytes, is_number_char);
                bytes.insert(0, b);

                let number = if is_float_number(&bytes) {
                    JsonValue::Float(convert_to_float(bytes)?)
                } else {
                    JsonValue::Int(convert_to_int(bytes)?)
                };

                if let Some(ref mut a) = array {
                    a.push(number);
                } else {
                    return Ok(number);
                }
            }
            ValueKind::String => {
                let bytes = read_until(iter_bytes, is_double_quote)?;

                if let Some(ref mut a) = array {
                    a.push(JsonValue::String(bytes_to_string(bytes)?));
                } else if object.is_some() {
                    bytes_buff.extend(bytes); // raw key bytes; decoded at NameSeparator
                } else {
                    return Ok(JsonValue::String(bytes_to_string(bytes)?));
                }
            }
        }
    }

    Err(ParseError::UnexpectedEof)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn read_while_returns_collected_bytes_and_leaves_delimiter() {
        let json = r#"800,"#;
        let bytes = json.as_bytes();
        let reader = BufReader::new(bytes);
        let mut iter_bytes = reader.bytes().peekable();

        let collected = read_while(&mut iter_bytes, is_number_char);

        assert_eq!(collected, vec![b'8', b'0', b'0']);
        assert!(matches!(iter_bytes.peek(), Some(Ok(b','))));
    }

    #[test]
    fn read_until_collects_body_and_consumes_delimiter() {
        let json = r#"Hello"world"#;
        let bytes = json.as_bytes();
        let reader = BufReader::new(bytes);
        let mut iter_bytes = reader.bytes().peekable();

        let collected = read_until(&mut iter_bytes, is_double_quote).unwrap();

        assert_eq!(collected, b"Hello".to_vec());
        assert!(matches!(iter_bytes.peek(), Some(Ok(b'w'))));
    }

    #[test]
    fn parse_valid_json_object() {
        let json = r#"
            {
                "Image": {
                    "Width":  800,
                    "Height": 600,
                    "Title":  "View from 15th Floor",
                    "Thumbnail": {
                        "Url": "http://www.example.com/image/481989943",
                        "Height": 125,
                        "Width":  100
                    },
                    "Animated" : false,
                    "IDs": [116, 943, 234, 38793]
                }
            }
            "#;

        let mut image_json_value = HashMap::<String, JsonValue>::new();
        image_json_value.insert(String::from("Width"), JsonValue::Int(800));
        image_json_value.insert(String::from("Height"), JsonValue::Int(600));
        image_json_value.insert(
            String::from("Title"),
            JsonValue::String(String::from("View from 15th Floor")),
        );

        let mut thumbnail_json_value = HashMap::<String, JsonValue>::new();
        thumbnail_json_value.insert(
            String::from("Url"),
            JsonValue::String(String::from("http://www.example.com/image/481989943")),
        );
        thumbnail_json_value.insert(String::from("Height"), JsonValue::Int(125));
        thumbnail_json_value.insert(String::from("Width"), JsonValue::Int(100));

        image_json_value.insert(
            String::from("Thumbnail"),
            JsonValue::Object(thumbnail_json_value),
        );
        image_json_value.insert(String::from("Animated"), JsonValue::Bool(false));
        image_json_value.insert(
            String::from("IDs"),
            JsonValue::Array(vec![
                JsonValue::Int(116),
                JsonValue::Int(943),
                JsonValue::Int(234),
                JsonValue::Int(38793),
            ]),
        );

        let mut root_json_value = HashMap::<String, JsonValue>::new();
        root_json_value.insert(String::from("Image"), JsonValue::Object(image_json_value));
        let expected_json_value = JsonValue::Object(root_json_value);

        let actual_json_value = parse(json).unwrap();
        assert_eq!(expected_json_value, actual_json_value);
    }

    #[test]
    fn parse_valid_json_array() {
        let json = r#"
            [
                {
                   "precision": "zip",
                   "Latitude":  37.7668,
                   "Longitude": -122.3959,
                   "Address":   "",
                   "City":      "SAN FRANCISCO",
                   "State":     "CA",
                   "Zip":       "94107",
                   "Country":   "US"
                },
                {
                   "precision": "zip",
                   "Latitude":  37.371991,
                   "Longitude": 37.371991,
                   "Address":   "",
                   "City":      "SUNNYVALE",
                   "State":     "CA",
                   "Zip":       "94085",
                   "Country":   "US"
                }
            ]
            "#;

        let mut first_object_map = HashMap::new();
        first_object_map.insert(
            String::from("precision"),
            JsonValue::String(String::from("zip")),
        );
        first_object_map.insert(String::from("Latitude"), JsonValue::Float(37.7668));
        first_object_map.insert(String::from("Longitude"), JsonValue::Float(-122.3959));
        first_object_map.insert(String::from("Address"), JsonValue::String(String::from("")));
        first_object_map.insert(
            String::from("City"),
            JsonValue::String(String::from("SAN FRANCISCO")),
        );
        first_object_map.insert(String::from("State"), JsonValue::String(String::from("CA")));
        first_object_map.insert(
            String::from("Zip"),
            JsonValue::String(String::from("94107")),
        );
        first_object_map.insert(
            String::from("Country"),
            JsonValue::String(String::from("US")),
        );

        let mut second_object_map = HashMap::new();
        second_object_map.insert(
            String::from("precision"),
            JsonValue::String(String::from("zip")),
        );
        second_object_map.insert(String::from("Latitude"), JsonValue::Float(37.371991));
        second_object_map.insert(String::from("Longitude"), JsonValue::Float(37.371991));
        second_object_map.insert(String::from("Address"), JsonValue::String(String::from("")));
        second_object_map.insert(
            String::from("City"),
            JsonValue::String(String::from("SUNNYVALE")),
        );
        second_object_map.insert(String::from("State"), JsonValue::String(String::from("CA")));
        second_object_map.insert(
            String::from("Zip"),
            JsonValue::String(String::from("94085")),
        );
        second_object_map.insert(
            String::from("Country"),
            JsonValue::String(String::from("US")),
        );

        let expected_json_value = JsonValue::Array(vec![
            JsonValue::Object(first_object_map),
            JsonValue::Object(second_object_map),
        ]);

        let actual_json_value = parse(json).unwrap();
        assert_eq!(expected_json_value, actual_json_value);
    }

    #[test]
    fn parse_string_with_escaped_quotes() {
        let json = r#""he said \"hi\"""#;
        let expected = JsonValue::String(String::from(r#"he said "hi""#));

        assert_eq!(expected, parse(json).unwrap());
    }

    #[test]
    fn parse_string_with_unicode_escape() {
        let json = r#""caf\u00e9""#;
        let expected = JsonValue::String(String::from("café"));

        assert_eq!(expected, parse(json).unwrap());
    }

    #[test]
    fn parse_rejects_invalid_escape() {
        // \x is not a valid JSON escape
        assert_eq!(parse(r#""\x""#), Err(ParseError::InvalidEscape(b'x')));
    }

    #[test]
    fn parse_rejects_invalid_unicode_escape() {
        // ZZZZ are not hex digits
        assert_eq!(parse(r#""\uZZZZ""#), Err(ParseError::InvalidUnicode));
    }

    #[test]
    fn parse_rejects_invalid_number() {
        // "1.2.3" isn't a parseable float; the wrapped ParseFloatError isn't
        // convenient to construct, so match the variant instead of asserting equality.
        assert!(matches!(parse("1.2.3"), Err(ParseError::InvalidFloat(_))));
    }

    #[test]
    fn parse_rejects_stray_closing_bracket() {
        // ']' with no open array
        assert_eq!(parse("]"), Err(ParseError::UnexpectedByte(b']')));
    }

    #[test]
    fn parse_rejects_unterminated_array() {
        // input ends mid-structure
        assert_eq!(parse("[1, 2"), Err(ParseError::UnexpectedEof));
    }

    #[test]
    fn parse_rejects_unterminated_string() {
        // closing quote never arrives
        assert_eq!(parse(r#""abc"#), Err(ParseError::UnexpectedEof));
    }

    #[test]
    fn parse_rejects_bad_keyword() {
        // "true" expected after 't', but 'a' breaks it
        assert_eq!(parse("tabc"), Err(ParseError::UnexpectedByte(b'a')));
    }

    #[test]
    fn parse_rejects_unexpected_byte() {
        // '@' is not a valid start of any JSON value
        assert_eq!(parse("@"), Err(ParseError::UnexpectedByte(b'@')));
    }

    #[test]
    fn parse_string_with_surrogate_pair() {
        // 😀 = U+1F600 = 😀
        let json = r#""\uD83D\uDE00""#;
        let expected = JsonValue::String(String::from("😀"));
        assert_eq!(expected, parse(json).unwrap());
    }

    #[test]
    fn parse_rejects_lone_low_surrogate() {
        assert_eq!(parse(r#""\uDE00""#), Err(ParseError::InvalidUnicode));
    }

    #[test]
    fn parse_rejects_broken_surrogate_pair() {
        // high surrogate not followed by \u
        assert_eq!(parse(r#""\uD83Dxx""#), Err(ParseError::InvalidUnicode));
    }
}

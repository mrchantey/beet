//! JSON text for any serde type, through beet's own reader and writer.
//!
//! A type serializes to a [`Value`], which writes as text, and text reads as a
//! [`Value`], which deserializes as the type: the text is exactly what
//! [`Value::to_json_string`] writes and [`Value::parse_json`] reads, so a
//! float survives every round trip and an object naming a key twice is
//! refused.
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
//! struct Point {
//! 	x: f64,
//! 	label: Option<String>,
//! }
//! let point = Point { x: 0.1, label: None };
//! let text = json_ext::to_string(&point).unwrap();
//! text.as_str().xpect_eq(r#"{"x":0.1,"label":null}"#);
//! json_ext::from_str::<Point>(&text).unwrap().xpect_eq(point);
//! ```
use crate::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// `value` as compact JSON text, a struct's fields in their order, see
/// [`Value::to_json_string`].
///
/// # Errors
/// Errors where `value` has no JSON form, ie a map keyed by a list.
pub fn to_string<T: ?Sized + Serialize>(value: &T) -> Result<String> {
	Value::from_serde(value)?.to_json_string().xok()
}

/// `value` as indented JSON text, see [`Value::to_json_pretty`].
///
/// # Errors
/// As [`to_string`].
pub fn to_string_pretty<T: ?Sized + Serialize>(value: &T) -> Result<String> {
	Value::from_serde(value)?.to_json_pretty().xok()
}

/// The bytes of [`to_string`].
///
/// # Errors
/// As [`to_string`].
pub fn to_vec<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>> {
	to_string(value)?.into_bytes().xok()
}

/// The bytes of [`to_string_pretty`].
///
/// # Errors
/// As [`to_string`].
pub fn to_vec_pretty<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>> {
	to_string_pretty(value)?.into_bytes().xok()
}

/// The `T` JSON `text` holds.
///
/// # Errors
/// Errors where `text` is not one JSON document, see [`Value::parse_json`], or
/// does not hold a `T`.
pub fn from_str<T: DeserializeOwned>(text: &str) -> Result<T> {
	Value::parse_json(text)?.into_serde()
}

/// The `T` JSON `bytes` hold.
///
/// # Errors
/// As [`from_str`], and where `bytes` are not utf8.
pub fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
	from_str(core::str::from_utf8(bytes)?)
}

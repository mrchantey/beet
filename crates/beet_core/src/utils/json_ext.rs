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
/// Errors with a [`JsonError`] where `text` is not one JSON document, see
/// [`Value::parse_json`], or does not hold a `T`, placed at the value that
/// does not fit and naming its path:
///
/// ```
/// # use beet_core::prelude::*;
/// #[derive(Debug, serde::Deserialize)]
/// struct Item {
/// 	count: u32,
/// }
/// let error = json_ext::from_str::<Vec<Item>>("[{\"count\": 1},\n{\"count\": \"two\"}]")
/// 	.unwrap_err();
/// let error = error.downcast_ref::<JsonError>().unwrap();
/// (error.line, error.column).xpect_eq((2, 11));
/// error.message.as_str().xpect_starts_with("`[1].count`: invalid type");
/// ```
pub fn from_str<T: DeserializeOwned>(text: &str) -> Result<T> {
	T::deserialize(ValueDeserializer::new(Value::parse_json(text)?)).map_err(
		|err| match JsonError::at_path(
			text,
			&err.path().into_inner(),
			err.to_string(),
		) {
			Some(placed) => placed.into(),
			None => err.into(),
		},
	)
}

/// The `T` JSON `bytes` hold.
///
/// # Errors
/// As [`from_str`], and where `bytes` are not utf8.
pub fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
	from_str(core::str::from_utf8(bytes)?)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use serde::Deserialize;

	#[derive(Debug, Deserialize)]
	#[allow(dead_code)]
	struct Item {
		count: u32,
		kind: Kind,
	}

	#[derive(Debug, Deserialize)]
	#[allow(dead_code)]
	enum Kind {
		Plain,
		Sized { width: u32 },
	}

	#[derive(Debug, Deserialize)]
	#[allow(dead_code)]
	#[serde(untagged)]
	enum Either {
		Number(u32),
		Text(String),
	}

	/// The line, column and message a read of `text` as a `T` fails with.
	fn place<T: serde::de::DeserializeOwned + core::fmt::Debug>(
		text: &str,
	) -> (usize, usize, String) {
		let error = json_ext::from_str::<T>(text).unwrap_err();
		let error = error.downcast_ref::<JsonError>().unwrap();
		(error.line, error.column, error.message.clone())
	}

	/// A type error is placed at the value that does not fit, and names its
	/// path, through lists, maps and enum variants alike.
	#[crate::test]
	fn places_type_errors() {
		let (line, column, message) = place::<Vec<Item>>(
			"[\n  {\"count\": 1, \"kind\": \"Plain\"},\n  {\"count\": -1, \"kind\": \"Plain\"}\n]",
		);
		(line, column).xpect_eq((3, 13));
		message.xpect_starts_with("`[1].count`: invalid value");
		// a missing field is missing from its object
		let (line, column, message) =
			place::<Vec<Item>>("[{\"count\": 1, \"kind\": \"Plain\"}, {}]");
		(line, column).xpect_eq((1, 33));
		message.xpect_contains("`[1]`: missing field");
		// a variant's payload is under the variant's name
		let (line, column, message) = place::<Item>(
			"{\"count\": 1, \"kind\": {\"Sized\": {\"width\": true}}}",
		);
		(line, column).xpect_eq((1, 42));
		message.xpect_starts_with("`kind.Sized.width`: invalid type");
		// serde buffers an untagged value whole, so the place is that value
		let (line, column, _) = place::<Vec<Either>>("[1, \"a\", true]");
		(line, column).xpect_eq((1, 10));
	}

	/// A syntax error is placed by the reader, before any type is asked.
	#[crate::test]
	fn places_syntax_errors() {
		let (line, column, message) = place::<Item>("{\n  \"count\": 1,\n}");
		(line, column).xpect_eq((3, 1));
		message.xpect_eq("expected `\"`");
	}
}

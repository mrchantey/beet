//! Module for the [`Value`] type
use crate::prelude::*;
use crate::types::value::reflect_ext;
use alloc::borrow::Cow;
use bevy::reflect::FromReflect;
use bevy::reflect::PartialReflect;
use bevy::reflect::Typed;

/// A map of string keys to [`Value`]s.
pub type Map = HashMap<SmolStr, Value>;

/// A json-like value type suitable for application level operations.
///
/// ## Floats
/// A wrapper around f64 that implements Eq and Hash by comparing the bit
/// representation of the float.
#[derive(Debug, Default, Clone, PartialEq, Reflect, Component)]
#[reflect(Default, Component)]
pub enum Value {
	/// A null value
	#[default]
	Null,
	/// A boolean value.
	Bool(bool),
	/// A signed integer value.
	Int(i64),
	/// An unsigned integer value.
	Uint(u64),
	/// A floating point value.
	Float(f64),
	/// A byte slice value.
	Bytes(Vec<u8>),
	/// A string value.
	Str(SmolStr),
	/// A map of string keys to values.
	Map(Map),
	/// An ordered list of values.
	List(Vec<Value>),
}



impl Eq for Value {}

/// The kind/type of a [`Value`], used in error reporting.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueKind {
	/// No value.
	Null,
	/// Boolean.
	Bool,
	/// Signed integer.
	Int,
	/// Unsigned integer.
	Uint,
	/// Floating point.
	Float,
	/// Byte slice.
	Bytes,
	/// String.
	Str,
	/// Key-value map.
	Map,
	/// Ordered list.
	List,
}

impl core::fmt::Display for ValueKind {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Null => write!(f, "null"),
			Self::Bool => write!(f, "bool"),
			Self::Int => write!(f, "int"),
			Self::Uint => write!(f, "uint"),
			Self::Float => write!(f, "float"),
			Self::Bytes => write!(f, "bytes"),
			Self::Str => write!(f, "str"),
			Self::Map => write!(f, "map"),
			Self::List => write!(f, "list"),
		}
	}
}

/// Error returned by [`Value`] accessor methods when the kind doesn't match.
#[derive(Debug, thiserror::Error)]
pub enum ValueError {
	/// The value was a different kind than expected.
	#[error("expected {expected}, got {actual}")]
	KindMismatch {
		/// The expected kind.
		expected: ValueKind,
		/// The actual kind.
		actual: ValueKind,
	},
}

impl Value {
	/// Creates a new value from anything convertible.
	pub fn new(value: impl Into<Self>) -> Self { value.into() }
	/// Creates a new string value.
	pub fn str(value: impl Into<SmolStr>) -> Self { Self::Str(value.into()) }

	/// Creates a new list value from an iterable of key-value pairs.
	pub fn new_list<T: Into<Value>>(
		values: impl IntoIterator<Item = T>,
	) -> Self {
		Self::List(values.into_iter().map(Into::into).collect())
	}

	/// Creates a new null value.
	pub fn null() -> Self { Self::Null }

	/// Creates a new empty map value.
	pub fn map() -> Self { Self::Map(Map::default()) }

	/// Creates a new empty list value.
	pub fn list() -> Self { Self::List(Vec::new()) }

	/// Returns `true` if this value is null.
	pub fn is_null(&self) -> bool { matches!(self, Self::Null) }

	/// Returns `true` if this value is a map.
	pub fn is_map(&self) -> bool { matches!(self, Self::Map(_)) }

	/// Returns `true` if this value is a list.
	pub fn is_list(&self) -> bool { matches!(self, Self::List(_)) }

	/// Returns the kind of this value.
	pub fn kind(&self) -> ValueKind {
		match self {
			Self::Null => ValueKind::Null,
			Self::Bool(_) => ValueKind::Bool,
			Self::Int(_) => ValueKind::Int,
			Self::Uint(_) => ValueKind::Uint,
			Self::Float(_) => ValueKind::Float,
			Self::Bytes(_) => ValueKind::Bytes,
			Self::Str(_) => ValueKind::Str,
			Self::Map(_) => ValueKind::Map,
			Self::List(_) => ValueKind::List,
		}
	}

	/// Returns this value as a map reference.
	pub fn as_map(&self) -> Result<&Map, ValueError> {
		match self {
			Self::Map(map) => Ok(map),
			other => Err(ValueError::KindMismatch {
				expected: ValueKind::Map,
				actual: other.kind(),
			}),
		}
	}

	/// Returns this value as a mutable map reference.
	pub fn as_map_mut(&mut self) -> Result<&mut Map, ValueError> {
		match self {
			Self::Map(map) => Ok(map),
			other => Err(ValueError::KindMismatch {
				expected: ValueKind::Map,
				actual: other.kind(),
			}),
		}
	}

	/// Returns this value as a list reference.
	pub fn as_list(&self) -> Result<&Vec<Value>, ValueError> {
		match self {
			Self::List(list) => Ok(list),
			other => Err(ValueError::KindMismatch {
				expected: ValueKind::List,
				actual: other.kind(),
			}),
		}
	}

	/// Returns this value as a mutable list reference.
	pub fn as_list_mut(&mut self) -> Result<&mut Vec<Value>, ValueError> {
		match self {
			Self::List(list) => Ok(list),
			other => Err(ValueError::KindMismatch {
				expected: ValueKind::List,
				actual: other.kind(),
			}),
		}
	}

	/// Returns this value as a bool, if it is one.
	pub fn as_bool(&self) -> Option<bool> {
		match self {
			Value::Bool(val) => Some(*val),
			_ => None,
		}
	}

	/// Returns this value as an i64, with cross-numeric coercion.
	pub fn as_i64(&self) -> Option<i64> {
		match self {
			Value::Int(val) => Some(*val),
			Value::Uint(val) => i64::try_from(*val).ok(),
			_ => None,
		}
	}

	/// Returns this value as a u64, with cross-numeric coercion.
	pub fn as_u64(&self) -> Option<u64> {
		match self {
			Value::Uint(val) => Some(*val),
			Value::Int(val) => u64::try_from(*val).ok(),
			_ => None,
		}
	}

	/// Returns this value as an f64, with numeric coercion.
	pub fn as_f64(&self) -> Option<f64> {
		match self {
			Value::Float(val) => Some(*val),
			Value::Int(val) => Some(*val as f64),
			Value::Uint(val) => Some(*val as f64),
			_ => None,
		}
	}

	/// Returns this value as a string reference, if it is one.
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Value::Str(val) => Some(val.as_str()),
			_ => None,
		}
	}

	/// Returns this value as a byte slice, if it is one.
	pub fn as_bytes(&self) -> Option<&[u8]> {
		match self {
			Value::Bytes(val) => Some(val),
			_ => None,
		}
	}

	/// Inserts a key-value pair into this map value.
	///
	/// Returns the previous value if the key existed.
	pub fn insert(
		&mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Result<Option<Value>> {
		self.as_map_mut()?.insert(key.into(), value.into()).xok()
	}

	/// Gets a value from a map by key.
	pub fn get(&self, key: &str) -> Option<&Value> {
		match self {
			Self::Map(map) => map.get(key),
			_ => None,
		}
	}

	/// Gets a mutable value from a map by key.
	pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
		match self {
			Self::Map(map) => map.get_mut(key),
			_ => None,
		}
	}

	/// Pushes a value onto this list.
	pub fn push(&mut self, value: impl Into<Value>) -> Result {
		self.as_list_mut()?.push(value.into()).xok()
	}

	/// Gets a value from a list by index.
	pub fn get_index(&self, index: usize) -> Option<&Value> {
		match self {
			Self::List(list) => list.get(index),
			_ => None,
		}
	}

	/// Gets a mutable value from a list by index.
	pub fn get_index_mut(&mut self, index: usize) -> Option<&mut Value> {
		match self {
			Self::List(list) => list.get_mut(index),
			_ => None,
		}
	}

	/// Optimistically parse a string into the most specific [`Value`] variant.
	///
	/// Attempts trimmed conversions in order:
	/// bool → uint → int → float (<18 chars) → string fallback (untrimmed).
	pub fn parse_string(input: &str) -> Self {
		let trimmed = input.trim();
		// strings greater than this in length will start
		// to lose precision, in that case parse as a string
		const MAX_F64_PRECISION: usize = 18;
		if let Ok(val) = trimmed.parse::<bool>() {
			Value::new(val)
		} else if let Ok(val) = trimmed.parse::<u64>() {
			Value::new(val)
		} else if let Ok(val) = trimmed.parse::<i64>() {
			Value::new(val)
		} else if trimmed.len() < MAX_F64_PRECISION
			&& let Ok(val) = trimmed.parse::<f64>()
		{
			Value::new(val)
		} else {
			Value::new(input)
		}
	}

	/// Convert a reflected type into a [`Value`].
	///
	/// Walks the reflection tree and builds a corresponding [`Value`] structure.
	pub fn from_reflect(reflect: &dyn PartialReflect) -> Result<Self> {
		reflect_ext::reflect_to_value(reflect)
	}

	/// Convert this [`Value`] into a concrete type using reflection.
	pub fn into_reflect<T>(&self) -> Result<T>
	where
		T: 'static + Send + Sync + FromReflect + Typed,
	{
		reflect_ext::value_to_type(self)
	}

	/// Convert from a [`serde_json::Value`].
	#[cfg(feature = "json")]
	pub fn from_json(json: serde_json::Value) -> Self {
		crate::types::value::json_ext::json_to_value(json)
	}

	/// Convert into a [`serde_json::Value`].
	#[cfg(feature = "json")]
	pub fn into_json(self) -> serde_json::Value {
		crate::types::value::json_ext::value_to_json(self)
	}
}

impl core::fmt::Display for Value {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Value::Null => write!(f, "null"),
			Value::Bool(val) => write!(f, "{}", val),
			Value::Int(val) => write!(f, "{}", val),
			Value::Uint(val) => write!(f, "{}", val),
			Value::Float(val) => write!(f, "{}", val),
			Value::Str(val) => write!(f, "{}", val),
			Value::Bytes(bytes) => {
				write!(
					f,
					"[{}]",
					bytes
						.iter()
						.map(|b| b.to_string())
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
			Value::List(list) => {
				write!(
					f,
					"[{}]",
					list.iter()
						.map(|v| v.to_string())
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
			Value::Map(map) => {
				let mut entries: Vec<_> = map.iter().collect();
				entries.sort_by_key(|(key, _)| key.as_str());
				write!(
					f,
					"{{{}}}",
					entries
						.iter()
						.map(|(key, val)| format!("{}: {}", key, val))
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
		}
	}
}

impl core::hash::Hash for Value {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Value::Null => {}
			Value::Bool(val) => val.hash(state),
			Value::Int(val) => val.hash(state),
			Value::Uint(val) => val.hash(state),
			Value::Float(val) => {
				val.to_bits().hash(state);
			}
			Value::Bytes(val) => val.hash(state),
			Value::Str(val) => val.hash(state),
			Value::Map(map) => {
				let mut entries: Vec<_> = map.iter().collect();
				entries.sort_by_key(|(key, _)| key.as_str());
				for (key, value) in entries {
					key.hash(state);
					value.hash(state);
				}
			}
			Value::List(list) => list.hash(state),
		}
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for Value {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		match self {
			Value::Null => serializer.serialize_unit(),
			Value::Bool(b) => serializer.serialize_bool(*b),
			Value::Int(i) => serializer.serialize_i64(*i),
			Value::Uint(u) => serializer.serialize_u64(*u),
			Value::Float(f) => serializer.serialize_f64(*f),
			Value::Bytes(bytes) => {
				use serde::ser::SerializeSeq;
				let mut seq = serializer.serialize_seq(Some(bytes.len()))?;
				for b in bytes {
					seq.serialize_element(b)?;
				}
				seq.end()
			}
			Value::Str(s) => serializer.serialize_str(s.as_str()),
			Value::List(list) => {
				use serde::ser::SerializeSeq;
				let mut seq = serializer.serialize_seq(Some(list.len()))?;
				for item in list {
					seq.serialize_element(item)?;
				}
				seq.end()
			}
			Value::Map(map) => {
				use serde::ser::SerializeMap;
				let mut sorted: Vec<_> = map.iter().collect();
				sorted.sort_by_key(|(k, _)| k.as_str());
				let mut m = serializer.serialize_map(Some(sorted.len()))?;
				for (k, v) in sorted {
					m.serialize_entry(k.as_str(), v)?;
				}
				m.end()
			}
		}
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Value {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		struct ValueVisitor;

		impl<'de> serde::de::Visitor<'de> for ValueVisitor {
			type Value = Value;

			fn expecting(
				&self,
				formatter: &mut core::fmt::Formatter<'_>,
			) -> core::fmt::Result {
				formatter.write_str("a JSON-like value")
			}

			fn visit_unit<E: serde::de::Error>(
				self,
			) -> core::result::Result<Value, E> {
				Ok(Value::Null)
			}

			fn visit_none<E: serde::de::Error>(
				self,
			) -> core::result::Result<Value, E> {
				Ok(Value::Null)
			}

			fn visit_bool<E: serde::de::Error>(
				self,
				v: bool,
			) -> core::result::Result<Value, E> {
				Ok(Value::Bool(v))
			}

			fn visit_i64<E: serde::de::Error>(
				self,
				v: i64,
			) -> core::result::Result<Value, E> {
				Ok(Value::Int(v))
			}

			fn visit_u64<E: serde::de::Error>(
				self,
				v: u64,
			) -> core::result::Result<Value, E> {
				Ok(Value::Uint(v))
			}

			fn visit_f64<E: serde::de::Error>(
				self,
				v: f64,
			) -> core::result::Result<Value, E> {
				Ok(Value::Float(v))
			}

			fn visit_str<E: serde::de::Error>(
				self,
				v: &str,
			) -> core::result::Result<Value, E> {
				Ok(Value::str(v))
			}

			fn visit_string<E: serde::de::Error>(
				self,
				v: String,
			) -> core::result::Result<Value, E> {
				Ok(Value::str(v))
			}

			fn visit_bytes<E: serde::de::Error>(
				self,
				v: &[u8],
			) -> core::result::Result<Value, E> {
				Ok(Value::Bytes(v.to_vec()))
			}

			fn visit_byte_buf<E: serde::de::Error>(
				self,
				v: Vec<u8>,
			) -> core::result::Result<Value, E> {
				Ok(Value::Bytes(v))
			}

			fn visit_seq<A: serde::de::SeqAccess<'de>>(
				self,
				mut seq: A,
			) -> core::result::Result<Value, A::Error> {
				let mut list = Vec::new();
				while let Some(elem) = seq.next_element()? {
					list.push(elem);
				}
				Ok(Value::List(list))
			}

			fn visit_map<A: serde::de::MapAccess<'de>>(
				self,
				mut map: A,
			) -> core::result::Result<Value, A::Error> {
				let mut result = Map::default();
				while let Some((key, value)) =
					map.next_entry::<SmolStr, Value>()?
				{
					result.insert(key, value);
				}
				Ok(Value::Map(result))
			}
		}

		deserializer.deserialize_any(ValueVisitor)
	}
}

impl From<bool> for Value {
	fn from(value: bool) -> Self { Value::Bool(value) }
}

impl From<i64> for Value {
	fn from(value: i64) -> Self { Value::Int(value) }
}

impl From<i32> for Value {
	fn from(value: i32) -> Self { Value::Int(value as i64) }
}

impl From<i16> for Value {
	fn from(value: i16) -> Self { Value::Int(value as i64) }
}

impl From<i8> for Value {
	fn from(value: i8) -> Self { Value::Int(value as i64) }
}

impl From<u64> for Value {
	fn from(value: u64) -> Self { Value::Uint(value) }
}

impl From<u32> for Value {
	fn from(value: u32) -> Self { Value::Uint(value as u64) }
}

impl From<u16> for Value {
	fn from(value: u16) -> Self { Value::Uint(value as u64) }
}

impl From<u8> for Value {
	fn from(value: u8) -> Self { Value::Uint(value as u64) }
}

impl From<f64> for Value {
	fn from(value: f64) -> Self { Value::Float(value) }
}

impl From<f32> for Value {
	fn from(value: f32) -> Self { Value::Float(value as f64) }
}

impl From<SmolStr> for Value {
	fn from(value: SmolStr) -> Self { Value::Str(value) }
}

impl From<String> for Value {
	fn from(value: String) -> Self { Value::str(value) }
}

impl From<&str> for Value {
	fn from(value: &str) -> Self { Value::str(value) }
}

impl<'a> From<Cow<'a, str>> for Value {
	fn from(value: Cow<'a, str>) -> Self { Value::str(value.as_ref()) }
}

impl From<Vec<u8>> for Value {
	fn from(value: Vec<u8>) -> Self { Value::Bytes(value) }
}

impl From<&[u8]> for Value {
	fn from(value: &[u8]) -> Self { Value::Bytes(value.to_vec()) }
}

impl From<Vec<Value>> for Value {
	fn from(value: Vec<Value>) -> Self { Value::List(value) }
}

impl<T: Into<Value>> From<Option<T>> for Value {
	fn from(value: Option<T>) -> Self {
		match value {
			Some(val) => val.into(),
			None => Self::Null,
		}
	}
}

impl<V: Into<Value>> From<HashMap<String, V>> for Value {
	fn from(value: HashMap<String, V>) -> Self {
		Self::Map(
			value
				.into_iter()
				.map(|(key, val)| (SmolStr::from(key), val.into()))
				.collect(),
		)
	}
}

impl<V: Into<Value>> From<HashMap<SmolStr, V>> for Value {
	fn from(value: HashMap<SmolStr, V>) -> Self {
		Self::Map(
			value
				.into_iter()
				.map(|(key, val)| (key, val.into()))
				.collect(),
		)
	}
}

// ── val! macro ──────────────────────────────────────────────────────

/// Creates a [`Value`] from a literal expression.
///
/// # Example
///
/// ```rust,no_run
/// # use beet_core::prelude::*;
/// let value = val!({
///     "name": "Alice",
///     "score": 100,
///     "items": [1, 2, 3],
///     "active": true
/// });
/// ```
#[macro_export]
macro_rules! val {
	// Null
	(null) => {
		$crate::prelude::Value::Null
	};

	// Boolean
	(true) => {
		$crate::prelude::Value::Bool(true)
	};
	(false) => {
		$crate::prelude::Value::Bool(false)
	};

	// Array
	([ $($elem:tt),* $(,)? ]) => {
		$crate::prelude::Value::List(vec![ $( $crate::val!($elem) ),* ])
	};

	// Object
	({ $($key:tt : $value:tt),* $(,)? }) => {
		{
			let mut map = $crate::prelude::HashMap::default();
			$(
				map.insert(
					$crate::prelude::SmolStr::from($key),
					$crate::val!($value),
				);
			)*
			$crate::prelude::Value::Map(map)
		}
	};

	// String literals
	($s:literal) => {
		$crate::prelude::Value::from($s)
	};

	// Other expressions (numbers, variables, etc.)
	($other:expr) => {
		$crate::prelude::Value::from($other)
	};
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn parse_string_bool() {
		Value::parse_string("true").xpect_eq(Value::Bool(true));
		Value::parse_string("false").xpect_eq(Value::Bool(false));
	}

	#[test]
	fn parse_string_uint() {
		Value::parse_string("42").xpect_eq(Value::Uint(42));
		Value::parse_string("0").xpect_eq(Value::Uint(0));
		Value::parse_string("007").xpect_eq(Value::Uint(7));
	}

	#[test]
	fn parse_string_int() {
		Value::parse_string("-7").xpect_eq(Value::Int(-7));
		Value::parse_string("-383").xpect_eq(Value::Int(-383));
	}

	#[test]
	fn parse_string_float() {
		Value::parse_string("3.14").xpect_eq(Value::Float(3.14));
		Value::parse_string("-383.484").xpect_eq(Value::Float(-383.484));
		Value::parse_string("0.0").xpect_eq(Value::Float(0.0));
	}

	#[test]
	fn parse_string_fallback() {
		for test_case in [
			"",
			"hello",
			"True",
			"-",
			".",
			"12abc",
			// too long number
			"2938297884738974328908",
		] {
			Value::parse_string(test_case).xpect_eq(Value::new(test_case));
		}
	}

	#[test]
	fn value_null_default() { Value::default().xpect_eq(Value::Null); }

	#[test]
	fn value_is_methods() {
		Value::Null.is_null().xpect_true();
		Value::Map(Map::default()).is_map().xpect_true();
		Value::List(Vec::new()).is_list().xpect_true();
	}

	#[test]
	fn value_from_primitives() {
		Value::from(true).xpect_eq(Value::Bool(true));
		Value::from("hello").xpect_eq(Value::Str("hello".into()));
		Value::from(42i64).xpect_eq(Value::Int(42));
		Value::from(42u64).xpect_eq(Value::Uint(42));
		Value::from(3.14f64).xpect_eq(Value::Float(3.14));
	}

	#[test]
	fn value_map_operations() {
		let mut val = Value::map();
		val.insert("key", "value").unwrap();
		val.get("key").unwrap().as_str().unwrap().xpect_eq("value");
	}

	#[test]
	fn value_list_operations() {
		let mut val = Value::list();
		val.push(1i64).unwrap();
		val.push(2i64).unwrap();
		val.get_index(0).unwrap().as_i64().unwrap().xpect_eq(1);
		val.get_index(1).unwrap().as_i64().unwrap().xpect_eq(2);
	}

	#[test]
	fn display_map() {
		let mut val = Value::map();
		val.insert("a", 1i64).unwrap();
		val.insert("b", 2i64).unwrap();
		val.to_string().xpect_eq("{a: 1, b: 2}");
	}

	#[test]
	fn display_list() {
		let mut val = Value::list();
		val.push(1i64).unwrap();
		val.push(2i64).unwrap();
		val.push(3i64).unwrap();
		val.to_string().xpect_eq("[1, 2, 3]");
	}

	#[test]
	fn val_macro_null() { val!(null).xpect_eq(Value::Null); }

	#[test]
	fn val_macro_bool() {
		val!(true).xpect_eq(Value::Bool(true));
		val!(false).xpect_eq(Value::Bool(false));
	}

	#[test]
	fn val_macro_string() {
		val!("hello").xpect_eq(Value::Str("hello".into()));
	}

	#[test]
	fn val_macro_number() { val!(42).xpect_eq(Value::Int(42)); }

	#[test]
	fn val_macro_array() {
		let value = val!([1, 2, 3]);
		let list = value.as_list().unwrap();
		list.len().xpect_eq(3);
	}

	#[test]
	fn val_macro_object() {
		let value = val!({
			"name": "Alice",
			"score": 100
		});
		let map = value.as_map().unwrap();
		map.get("name").unwrap().as_str().unwrap().xpect_eq("Alice");
		map.get("score").unwrap().as_i64().unwrap().xpect_eq(100);
	}

	#[test]
	fn val_macro_nested() {
		let value = val!({
			"user": {
				"name": "Bob"
			},
			"items": [1, 2, 3]
		});
		let map = value.as_map().unwrap();
		let user = map.get("user").unwrap().as_map().unwrap();
		user.get("name").unwrap().as_str().unwrap().xpect_eq("Bob");
	}

	#[cfg(feature = "std")]
	#[test]
	fn value_hash_consistency() {
		use std::hash::DefaultHasher;
		use std::hash::Hash;
		use std::hash::Hasher;

		let val1 = val!({"a": 1, "b": 2});
		let val2 = val!({"a": 1, "b": 2});

		let mut hasher1 = DefaultHasher::new();
		let mut hasher2 = DefaultHasher::new();

		val1.hash(&mut hasher1);
		val2.hash(&mut hasher2);

		hasher1.finish().xpect_eq(hasher2.finish());
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct AllNumericTypes {
		signed_8: i8,
		signed_16: i16,
		signed_32: i32,
		signed_64: i64,
		unsigned_8: u8,
		unsigned_16: u16,
		unsigned_32: u32,
		unsigned_64: u64,
		float_32: f32,
		float_64: f64,
	}

	#[test]
	fn roundtrip_all_numeric_types() {
		let original = AllNumericTypes {
			signed_8: -8,
			signed_16: -16,
			signed_32: -32,
			signed_64: -64,
			unsigned_8: 8,
			unsigned_16: 16,
			unsigned_32: 32,
			unsigned_64: 64,
			float_32: 3.14,
			float_64: 2.718,
		};

		let value = Value::from_reflect(&original).unwrap();
		let result: AllNumericTypes = value.into_reflect().unwrap();

		result.signed_8.xpect_eq(original.signed_8);
		result.signed_16.xpect_eq(original.signed_16);
		result.signed_32.xpect_eq(original.signed_32);
		result.signed_64.xpect_eq(original.signed_64);
		result.unsigned_8.xpect_eq(original.unsigned_8);
		result.unsigned_16.xpect_eq(original.unsigned_16);
		result.unsigned_32.xpect_eq(original.unsigned_32);
		result.unsigned_64.xpect_eq(original.unsigned_64);
		((result.float_32 - original.float_32).abs() < 0.001).xpect_true();
		((result.float_64 - original.float_64).abs() < 0.001).xpect_true();
	}

	#[test]
	fn from_option_impls() {
		Value::from(Some(42i64)).xpect_eq(Value::Int(42));
		Value::from(None::<i64>).xpect_eq(Value::Null);
	}

	#[test]
	fn from_hashmap_impl() {
		let mut input: HashMap<String, i64> = HashMap::default();
		input.insert("key".to_string(), 42i64);
		let value = Value::from(input);
		value.is_map().xpect_true();
		value.get("key").unwrap().as_i64().unwrap().xpect_eq(42);
	}
}

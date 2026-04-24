//! Serde utilities for [`Value`]
use crate::prelude::*;


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





/// Converts a [`serde_json::Value`] to a [`Value`], preserving numeric precision.
///
/// Maps JSON types directly: null→Null, bool→Bool, numbers try u64 then i64 then f64,
/// string→Str, array→List, object→Map.
#[cfg(feature = "json")]
pub fn json_to_value(json: serde_json::Value) -> Value {
	match json {
		serde_json::Value::Null => Value::Null,
		serde_json::Value::Bool(b) => Value::Bool(b),
		serde_json::Value::Number(n) => {
			if let Some(u) = n.as_u64() {
				Value::Uint(u)
			} else if let Some(i) = n.as_i64() {
				Value::Int(i)
			} else if let Some(f) = n.as_f64() {
				Value::Float(f)
			} else {
				Value::Null
			}
		}
		serde_json::Value::String(s) => Value::Str(s.into()),
		serde_json::Value::Array(arr) => {
			Value::List(arr.into_iter().map(json_to_value).collect())
		}
		serde_json::Value::Object(obj) => Value::Map(
			obj.into_iter()
				.map(|(k, v)| (SmolStr::from(k), json_to_value(v)))
				.collect(),
		),
	}
}

/// Converts a [`Value`] to a [`serde_json::Value`].
///
/// Bytes are encoded as a JSON array of integers.
#[cfg(feature = "json")]
pub fn value_to_json(value: Value) -> serde_json::Value {
	match value {
		Value::Null => serde_json::Value::Null,
		Value::Bool(b) => serde_json::Value::Bool(b),
		Value::Int(i) => serde_json::Value::Number(i.into()),
		Value::Uint(u) => serde_json::Value::Number(u.into()),
		Value::Float(f) => serde_json::Number::from_f64(f)
			.map(serde_json::Value::Number)
			.unwrap_or(serde_json::Value::Null),
		Value::Bytes(bytes) => serde_json::Value::Array(
			bytes
				.into_iter()
				.map(|b| serde_json::Value::Number(b.into()))
				.collect(),
		),
		Value::Str(s) => serde_json::Value::String(s.into()),
		Value::List(list) => serde_json::Value::Array(
			list.into_iter().map(value_to_json).collect(),
		),
		Value::Map(map) => serde_json::Value::Object(
			map.into_iter()
				.map(|(k, v)| (k.into(), value_to_json(v)))
				.collect(),
		),
	}
}

#[cfg(feature = "json")]
impl From<serde_json::Value> for Value {
	fn from(json: serde_json::Value) -> Self { json_to_value(json) }
}

#[cfg(feature = "json")]
impl From<Value> for serde_json::Value {
	fn from(value: Value) -> Self { value_to_json(value) }
}

#[cfg(feature = "json")]
impl From<Schema> for serde_json::Value {
	fn from(schema: Schema) -> Self { value_to_json(schema.into_inner()) }
}

#[cfg(feature = "json")]
impl From<serde_json::Value> for Schema {
	fn from(json: serde_json::Value) -> Self {
		Schema::from_value(json_to_value(json))
	}
}

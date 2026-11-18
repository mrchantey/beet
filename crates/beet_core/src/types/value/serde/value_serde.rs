//! Serde representation for [`Value`].
//!
//! Two forms, switched on `is_human_readable`. A self describing format
//! (json, ron, a [`Value`] itself) takes the value as it reads, `"hi"` or
//! `{"a": 1}`, and reads it back by what it finds. A binary format that cannot
//! say what comes next (postcard) takes it tagged, each value an enum variant
//! naming its kind, so it reads back without `deserialize_any`.
use crate::prelude::*;

/// The tagged form a [`Value`] is written in, see the [module docs](self).
#[derive(::serde::Serialize)]
#[serde(rename = "Value")]
enum TaggedRef<'a> {
	Null,
	Bool(bool),
	Int(i64),
	Uint(u64),
	Float(f64),
	Bytes(BytesRef<'a>),
	Str(&'a str),
	Map(&'a Map),
	List(&'a [Value]),
}

/// The tagged form a [`Value`] is read from, variant for variant
/// [`TaggedRef`].
#[derive(::serde::Deserialize)]
#[serde(rename = "Value")]
enum Tagged {
	Null,
	Bool(bool),
	Int(i64),
	Uint(u64),
	Float(f64),
	Bytes(ByteBuf),
	Str(SmolStr),
	Map(Map),
	List(Vec<Value>),
}

/// Bytes written as a byte string, never a sequence of numbers.
struct BytesRef<'a>(&'a [u8]);

impl ::serde::Serialize for BytesRef<'_> {
	fn serialize<S: ::serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_bytes(self.0)
	}
}

/// Bytes read from a byte string.
struct ByteBuf(Vec<u8>);

impl<'de> ::serde::Deserialize<'de> for ByteBuf {
	fn deserialize<D: ::serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		struct ByteBufVisitor;

		impl<'de> ::serde::de::Visitor<'de> for ByteBufVisitor {
			type Value = ByteBuf;

			fn expecting(
				&self,
				formatter: &mut core::fmt::Formatter<'_>,
			) -> core::fmt::Result {
				formatter.write_str("a byte string")
			}

			fn visit_bytes<E: ::serde::de::Error>(
				self,
				v: &[u8],
			) -> core::result::Result<ByteBuf, E> {
				Ok(ByteBuf(v.to_vec()))
			}

			fn visit_byte_buf<E: ::serde::de::Error>(
				self,
				v: Vec<u8>,
			) -> core::result::Result<ByteBuf, E> {
				Ok(ByteBuf(v))
			}
		}

		deserializer.deserialize_byte_buf(ByteBufVisitor)
	}
}

impl From<Tagged> for Value {
	fn from(tagged: Tagged) -> Self {
		match tagged {
			Tagged::Null => Value::Null,
			Tagged::Bool(value) => Value::Bool(value),
			Tagged::Int(value) => Value::Int(value),
			Tagged::Uint(value) => Value::Uint(value),
			Tagged::Float(value) => Value::Float(value),
			Tagged::Bytes(value) => Value::Bytes(value.0),
			Tagged::Str(value) => Value::Str(value),
			Tagged::Map(value) => Value::Map(value),
			Tagged::List(value) => Value::List(value),
		}
	}
}

impl<'a> From<&'a Value> for TaggedRef<'a> {
	fn from(value: &'a Value) -> Self {
		match value {
			Value::Null => TaggedRef::Null,
			Value::Bool(value) => TaggedRef::Bool(*value),
			Value::Int(value) => TaggedRef::Int(*value),
			Value::Uint(value) => TaggedRef::Uint(*value),
			Value::Float(value) => TaggedRef::Float(*value),
			Value::Bytes(value) => TaggedRef::Bytes(BytesRef(value)),
			Value::Str(value) => TaggedRef::Str(value.as_str()),
			Value::Map(value) => TaggedRef::Map(value),
			Value::List(value) => TaggedRef::List(value),
		}
	}
}

impl ::serde::Serialize for Value {
	fn serialize<S: ::serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		if !serializer.is_human_readable() {
			return TaggedRef::from(self).serialize(serializer);
		}
		match self {
			Value::Null => serializer.serialize_unit(),
			Value::Bool(b) => serializer.serialize_bool(*b),
			Value::Int(i) => serializer.serialize_i64(*i),
			Value::Uint(u) => serializer.serialize_u64(*u),
			Value::Float(f) => serializer.serialize_f64(*f),
			// a byte string, which json writes as the same array of numbers a
			// seq would, while a format with bytes (DAG-CBOR, postcard, a
			// `Value` itself) keeps them bytes
			Value::Bytes(bytes) => serializer.serialize_bytes(bytes),
			Value::Str(s) => serializer.serialize_str(s.as_str()),
			Value::List(list) => {
				use ::serde::ser::SerializeSeq;
				let mut seq = serializer.serialize_seq(Some(list.len()))?;
				for item in list {
					seq.serialize_element(item)?;
				}
				seq.end()
			}
			Value::Map(map) => {
				use ::serde::ser::SerializeMap;
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

impl<'de> ::serde::Deserialize<'de> for Value {
	fn deserialize<D: ::serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		if !deserializer.is_human_readable() {
			return Tagged::deserialize(deserializer).map(Value::from);
		}
		struct ValueVisitor;

		impl<'de> ::serde::de::Visitor<'de> for ValueVisitor {
			type Value = Value;

			fn expecting(
				&self,
				formatter: &mut core::fmt::Formatter<'_>,
			) -> core::fmt::Result {
				formatter.write_str("a JSON-like value")
			}

			fn visit_unit<E: ::serde::de::Error>(
				self,
			) -> core::result::Result<Value, E> {
				Ok(Value::Null)
			}

			fn visit_none<E: ::serde::de::Error>(
				self,
			) -> core::result::Result<Value, E> {
				Ok(Value::Null)
			}

			fn visit_bool<E: ::serde::de::Error>(
				self,
				v: bool,
			) -> core::result::Result<Value, E> {
				Ok(Value::Bool(v))
			}

			fn visit_i64<E: ::serde::de::Error>(
				self,
				v: i64,
			) -> core::result::Result<Value, E> {
				Ok(Value::Int(v))
			}

			fn visit_u64<E: ::serde::de::Error>(
				self,
				v: u64,
			) -> core::result::Result<Value, E> {
				Ok(Value::Uint(v))
			}

			fn visit_f64<E: ::serde::de::Error>(
				self,
				v: f64,
			) -> core::result::Result<Value, E> {
				Ok(Value::Float(v))
			}

			fn visit_str<E: ::serde::de::Error>(
				self,
				v: &str,
			) -> core::result::Result<Value, E> {
				Ok(Value::str(v))
			}

			fn visit_string<E: ::serde::de::Error>(
				self,
				v: String,
			) -> core::result::Result<Value, E> {
				Ok(Value::str(v))
			}

			fn visit_bytes<E: ::serde::de::Error>(
				self,
				v: &[u8],
			) -> core::result::Result<Value, E> {
				Ok(Value::Bytes(v.to_vec()))
			}

			fn visit_byte_buf<E: ::serde::de::Error>(
				self,
				v: Vec<u8>,
			) -> core::result::Result<Value, E> {
				Ok(Value::Bytes(v))
			}

			fn visit_seq<A: ::serde::de::SeqAccess<'de>>(
				self,
				mut seq: A,
			) -> core::result::Result<Value, A::Error> {
				let mut list = Vec::new();
				while let Some(elem) = seq.next_element()? {
					list.push(elem);
				}
				Ok(Value::List(list))
			}

			fn visit_map<A: ::serde::de::MapAccess<'de>>(
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

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	/// A binary format that cannot `deserialize_any` reads every kind back,
	/// bytes as bytes and a nested map whole.
	#[cfg(feature = "postcard")]
	#[crate::test]
	fn round_trips_a_binary_format() {
		let value = Value::Map(Map::new([
			("null", Value::Null),
			("bool", Value::Bool(true)),
			("int", Value::Int(-3)),
			("uint", Value::Uint(u64::MAX)),
			("float", Value::Float(1.5)),
			("bytes", Value::Bytes(vec![0, 255])),
			("str", Value::str("hi")),
			("list", Value::new_list([1u64, 2])),
			("map", Value::Map(Map::new([("z", 1u64), ("a", 2)]))),
		]));
		let bytes = postcard::to_allocvec(&value).unwrap();
		postcard::from_bytes::<Value>(&bytes)
			.unwrap()
			.xpect_eq(value);
	}

	#[crate::test]
	fn serializes_maps_in_key_order() {
		let mut map = Map::default();
		map.insert("z", 1_i64);
		map.insert("a", 2_i64);

		serde_json::to_string(&Value::Map(map))
			.unwrap()
			.xpect_eq(r#"{"a":2,"z":1}"#.to_string());
	}
}

//! Serde representation for [`Value`].
use crate::prelude::*;

impl ::serde::Serialize for Value {
	fn serialize<S: ::serde::Serializer>(
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
				use ::serde::ser::SerializeSeq;
				let mut seq = serializer.serialize_seq(Some(bytes.len()))?;
				for b in bytes {
					seq.serialize_element(b)?;
				}
				seq.end()
			}
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

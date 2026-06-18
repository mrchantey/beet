//! Serde utilities for [`Value`]
use crate::prelude::*;

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

/// A [`serde::Serializer`] that builds a [`Value`] directly.
///
/// This exists so [`Value::from_serde`] does not have to round-trip through
/// `serde_json::Value`, whose `Number` type collapses the signed/unsigned
/// distinction (a positive `i32` would otherwise come back as
/// [`Value::Uint`]). Mapping mirrors `serde_json` in every other respect.
pub use ser::ValueSerializer;

mod ser {
	use super::*;
	use alloc::string::String;
	use alloc::string::ToString;
	use serde::Serialize;
	use serde::ser;

	#[derive(Debug)]
	pub struct SerError(String);
	impl core::fmt::Display for SerError {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str(&self.0)
		}
	}
	impl core::error::Error for SerError {}
	impl ser::Error for SerError {
		fn custom<T: core::fmt::Display>(msg: T) -> Self {
			SerError(msg.to_string())
		}
	}

	type SerResult<T = Value> = core::result::Result<T, SerError>;

	fn key_to_smolstr(value: Value) -> SerResult<SmolStr> {
		match value {
			Value::Str(s) => Ok(s),
			Value::Int(i) => Ok(i.to_string().into()),
			Value::Uint(u) => Ok(u.to_string().into()),
			Value::Bool(b) => Ok(b.to_string().into()),
			other => Err(SerError(format!(
				"map keys must be string-like, got {:?}",
				other.kind()
			))),
		}
	}

	pub struct ValueSerializer;

	impl ser::Serializer for ValueSerializer {
		type Ok = Value;
		type Error = SerError;
		type SerializeSeq = SeqSer;
		type SerializeTuple = SeqSer;
		type SerializeTupleStruct = SeqSer;
		type SerializeTupleVariant = VariantSeqSer;
		type SerializeMap = MapSer;
		type SerializeStruct = StructSer;
		type SerializeStructVariant = VariantStructSer;

		fn serialize_bool(self, v: bool) -> SerResult { Ok(Value::Bool(v)) }
		fn serialize_i8(self, v: i8) -> SerResult { Ok(Value::Int(v as i64)) }
		fn serialize_i16(self, v: i16) -> SerResult { Ok(Value::Int(v as i64)) }
		fn serialize_i32(self, v: i32) -> SerResult { Ok(Value::Int(v as i64)) }
		fn serialize_i64(self, v: i64) -> SerResult { Ok(Value::Int(v)) }
		fn serialize_u8(self, v: u8) -> SerResult { Ok(Value::Uint(v as u64)) }
		fn serialize_u16(self, v: u16) -> SerResult {
			Ok(Value::Uint(v as u64))
		}
		fn serialize_u32(self, v: u32) -> SerResult {
			Ok(Value::Uint(v as u64))
		}
		fn serialize_u64(self, v: u64) -> SerResult { Ok(Value::Uint(v)) }
		fn serialize_f32(self, v: f32) -> SerResult {
			Ok(Value::Float(v as f64))
		}
		fn serialize_f64(self, v: f64) -> SerResult { Ok(Value::Float(v)) }
		fn serialize_char(self, v: char) -> SerResult {
			Ok(Value::str(v.to_string()))
		}
		fn serialize_str(self, v: &str) -> SerResult { Ok(Value::str(v)) }
		fn serialize_bytes(self, v: &[u8]) -> SerResult {
			Ok(Value::Bytes(v.to_vec()))
		}
		fn serialize_none(self) -> SerResult { Ok(Value::Null) }
		fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> SerResult {
			v.serialize(self)
		}
		fn serialize_unit(self) -> SerResult { Ok(Value::Null) }
		fn serialize_unit_struct(self, _name: &'static str) -> SerResult {
			Ok(Value::Null)
		}
		fn serialize_unit_variant(
			self,
			_name: &'static str,
			_idx: u32,
			variant: &'static str,
		) -> SerResult {
			Ok(Value::str(variant))
		}
		fn serialize_newtype_struct<T: ?Sized + Serialize>(
			self,
			_name: &'static str,
			v: &T,
		) -> SerResult {
			v.serialize(self)
		}
		fn serialize_newtype_variant<T: ?Sized + Serialize>(
			self,
			_name: &'static str,
			_idx: u32,
			variant: &'static str,
			v: &T,
		) -> SerResult {
			let mut map = Map::default();
			map.insert(variant, v.serialize(ValueSerializer)?);
			Ok(Value::Map(map))
		}
		fn serialize_seq(self, _len: Option<usize>) -> SerResult<SeqSer> {
			Ok(SeqSer { items: Vec::new() })
		}
		fn serialize_tuple(self, len: usize) -> SerResult<SeqSer> {
			self.serialize_seq(Some(len))
		}
		fn serialize_tuple_struct(
			self,
			_name: &'static str,
			len: usize,
		) -> SerResult<SeqSer> {
			self.serialize_seq(Some(len))
		}
		fn serialize_tuple_variant(
			self,
			_name: &'static str,
			_idx: u32,
			variant: &'static str,
			_len: usize,
		) -> SerResult<VariantSeqSer> {
			Ok(VariantSeqSer {
				variant,
				items: Vec::new(),
			})
		}
		fn serialize_map(self, _len: Option<usize>) -> SerResult<MapSer> {
			Ok(MapSer {
				map: Map::default(),
				next_key: None,
			})
		}
		fn serialize_struct(
			self,
			_name: &'static str,
			_len: usize,
		) -> SerResult<StructSer> {
			Ok(StructSer {
				map: Map::default(),
			})
		}
		fn serialize_struct_variant(
			self,
			_name: &'static str,
			_idx: u32,
			variant: &'static str,
			_len: usize,
		) -> SerResult<VariantStructSer> {
			Ok(VariantStructSer {
				variant,
				map: Map::default(),
			})
		}
	}

	pub struct SeqSer {
		items: Vec<Value>,
	}
	impl ser::SerializeSeq for SeqSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_element<T: ?Sized + Serialize>(
			&mut self,
			v: &T,
		) -> SerResult<()> {
			self.items.push(v.serialize(ValueSerializer)?);
			Ok(())
		}
		fn end(self) -> SerResult { Ok(Value::List(self.items)) }
	}
	impl ser::SerializeTuple for SeqSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_element<T: ?Sized + Serialize>(
			&mut self,
			v: &T,
		) -> SerResult<()> {
			ser::SerializeSeq::serialize_element(self, v)
		}
		fn end(self) -> SerResult { ser::SerializeSeq::end(self) }
	}
	impl ser::SerializeTupleStruct for SeqSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_field<T: ?Sized + Serialize>(
			&mut self,
			v: &T,
		) -> SerResult<()> {
			ser::SerializeSeq::serialize_element(self, v)
		}
		fn end(self) -> SerResult { ser::SerializeSeq::end(self) }
	}

	pub struct VariantSeqSer {
		variant: &'static str,
		items: Vec<Value>,
	}
	impl ser::SerializeTupleVariant for VariantSeqSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_field<T: ?Sized + Serialize>(
			&mut self,
			v: &T,
		) -> SerResult<()> {
			self.items.push(v.serialize(ValueSerializer)?);
			Ok(())
		}
		fn end(self) -> SerResult {
			let mut map = Map::default();
			map.insert(self.variant, Value::List(self.items));
			Ok(Value::Map(map))
		}
	}

	pub struct MapSer {
		map: Map,
		next_key: Option<SmolStr>,
	}
	impl ser::SerializeMap for MapSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_key<T: ?Sized + Serialize>(
			&mut self,
			key: &T,
		) -> SerResult<()> {
			self.next_key =
				Some(key_to_smolstr(key.serialize(ValueSerializer)?)?);
			Ok(())
		}
		fn serialize_value<T: ?Sized + Serialize>(
			&mut self,
			v: &T,
		) -> SerResult<()> {
			let key = self.next_key.take().ok_or_else(|| {
				SerError("serialize_value called before serialize_key".into())
			})?;
			self.map.insert(key, v.serialize(ValueSerializer)?);
			Ok(())
		}
		fn end(self) -> SerResult { Ok(Value::Map(self.map)) }
	}

	pub struct StructSer {
		map: Map,
	}
	impl ser::SerializeStruct for StructSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_field<T: ?Sized + Serialize>(
			&mut self,
			key: &'static str,
			v: &T,
		) -> SerResult<()> {
			self.map.insert(key, v.serialize(ValueSerializer)?);
			Ok(())
		}
		fn end(self) -> SerResult { Ok(Value::Map(self.map)) }
	}

	pub struct VariantStructSer {
		variant: &'static str,
		map: Map,
	}
	impl ser::SerializeStructVariant for VariantStructSer {
		type Ok = Value;
		type Error = SerError;
		fn serialize_field<T: ?Sized + Serialize>(
			&mut self,
			key: &'static str,
			v: &T,
		) -> SerResult<()> {
			self.map.insert(key, v.serialize(ValueSerializer)?);
			Ok(())
		}
		fn end(self) -> SerResult {
			let mut outer = Map::default();
			outer.insert(self.variant, Value::Map(self.map));
			Ok(Value::Map(outer))
		}
	}
}

/// A [`serde::Deserializer`] that reads a [`Value`] directly.
///
/// The counterpart to [`ValueSerializer`], so [`Value::into_serde`] does not
/// have to round-trip through `serde_json::Value` either.
pub use de::ValueDeserializer;

mod de {
	use super::*;
	use alloc::string::String;
	use alloc::string::ToString;
	use serde::de;
	use serde::de::IntoDeserializer;

	#[derive(Debug)]
	pub struct DeError(String);
	impl core::fmt::Display for DeError {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str(&self.0)
		}
	}
	impl core::error::Error for DeError {}
	impl de::Error for DeError {
		fn custom<T: core::fmt::Display>(msg: T) -> Self {
			DeError(msg.to_string())
		}
	}

	type DeResult<T> = core::result::Result<T, DeError>;

	pub struct ValueDeserializer {
		value: Value,
	}
	impl ValueDeserializer {
		pub fn new(value: Value) -> Self { Self { value } }
	}

	impl<'de> de::Deserializer<'de> for ValueDeserializer {
		type Error = DeError;

		fn deserialize_any<V: de::Visitor<'de>>(
			self,
			visitor: V,
		) -> DeResult<V::Value> {
			match self.value {
				Value::Null => visitor.visit_unit(),
				Value::Bool(b) => visitor.visit_bool(b),
				Value::Int(i) => visitor.visit_i64(i),
				Value::Uint(u) => visitor.visit_u64(u),
				Value::Float(f) => visitor.visit_f64(f),
				Value::Bytes(b) => visitor.visit_byte_buf(b),
				Value::Str(s) => visitor.visit_string(s.to_string()),
				Value::List(list) => visitor.visit_seq(SeqAccess {
					iter: list.into_iter(),
				}),
				Value::Map(map) => visitor.visit_map(MapAccess {
					iter: map.into_iter(),
					value: None,
				}),
			}
		}

		fn deserialize_option<V: de::Visitor<'de>>(
			self,
			visitor: V,
		) -> DeResult<V::Value> {
			match self.value {
				Value::Null => visitor.visit_none(),
				_ => visitor.visit_some(self),
			}
		}

		fn deserialize_newtype_struct<V: de::Visitor<'de>>(
			self,
			_name: &'static str,
			visitor: V,
		) -> DeResult<V::Value> {
			visitor.visit_newtype_struct(self)
		}

		fn deserialize_enum<V: de::Visitor<'de>>(
			self,
			_name: &'static str,
			_variants: &'static [&'static str],
			visitor: V,
		) -> DeResult<V::Value> {
			let (variant, payload) = match self.value {
				// unit variants serialize as a bare string
				Value::Str(s) => (s, None),
				// all other variants serialize as a single-entry map
				Value::Map(map) => {
					let mut iter = map.into_iter();
					let entry = iter.next().ok_or_else(|| {
						DeError("expected a single-entry map for enum".into())
					})?;
					if iter.next().is_some() {
						return Err(DeError(
							"expected a single-entry map for enum".into(),
						));
					}
					(entry.0, Some(entry.1))
				}
				other => {
					return Err(DeError(format!(
						"cannot deserialize enum from {:?}",
						other.kind()
					)));
				}
			};
			visitor.visit_enum(EnumAccess { variant, payload })
		}

		serde::forward_to_deserialize_any! {
			bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
			string bytes byte_buf unit unit_struct seq tuple tuple_struct
			map struct identifier ignored_any
		}
	}

	struct SeqAccess {
		iter: alloc::vec::IntoIter<Value>,
	}
	impl<'de> de::SeqAccess<'de> for SeqAccess {
		type Error = DeError;
		fn next_element_seed<T: de::DeserializeSeed<'de>>(
			&mut self,
			seed: T,
		) -> DeResult<Option<T::Value>> {
			match self.iter.next() {
				Some(value) => {
					seed.deserialize(ValueDeserializer::new(value)).map(Some)
				}
				None => Ok(None),
			}
		}
	}

	struct MapAccess {
		iter: <Map as IntoIterator>::IntoIter,
		value: Option<Value>,
	}
	impl<'de> de::MapAccess<'de> for MapAccess {
		type Error = DeError;
		fn next_key_seed<K: de::DeserializeSeed<'de>>(
			&mut self,
			seed: K,
		) -> DeResult<Option<K::Value>> {
			match self.iter.next() {
				Some((key, value)) => {
					self.value = Some(value);
					seed.deserialize(key.as_str().into_deserializer()).map(Some)
				}
				None => Ok(None),
			}
		}
		fn next_value_seed<V: de::DeserializeSeed<'de>>(
			&mut self,
			seed: V,
		) -> DeResult<V::Value> {
			let value = self.value.take().ok_or_else(|| {
				DeError("next_value called before next_key".into())
			})?;
			seed.deserialize(ValueDeserializer::new(value))
		}
	}

	struct EnumAccess {
		variant: SmolStr,
		payload: Option<Value>,
	}
	impl<'de> de::EnumAccess<'de> for EnumAccess {
		type Error = DeError;
		type Variant = VariantAccess;
		fn variant_seed<V: de::DeserializeSeed<'de>>(
			self,
			seed: V,
		) -> DeResult<(V::Value, Self::Variant)> {
			let variant =
				seed.deserialize(self.variant.as_str().into_deserializer())?;
			Ok((variant, VariantAccess {
				payload: self.payload,
			}))
		}
	}

	struct VariantAccess {
		payload: Option<Value>,
	}
	impl<'de> de::VariantAccess<'de> for VariantAccess {
		type Error = DeError;
		fn unit_variant(self) -> DeResult<()> {
			match self.payload {
				None => Ok(()),
				Some(_) => {
					Err(DeError("expected unit variant, found payload".into()))
				}
			}
		}
		fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
			self,
			seed: T,
		) -> DeResult<T::Value> {
			seed.deserialize(ValueDeserializer::new(self.require_payload()?))
		}
		fn tuple_variant<V: de::Visitor<'de>>(
			self,
			_len: usize,
			visitor: V,
		) -> DeResult<V::Value> {
			match self.require_payload()? {
				Value::List(list) => visitor.visit_seq(SeqAccess {
					iter: list.into_iter(),
				}),
				other => Err(DeError(format!(
					"expected a list for tuple variant, got {:?}",
					other.kind()
				))),
			}
		}
		fn struct_variant<V: de::Visitor<'de>>(
			self,
			_fields: &'static [&'static str],
			visitor: V,
		) -> DeResult<V::Value> {
			match self.require_payload()? {
				Value::Map(map) => visitor.visit_map(MapAccess {
					iter: map.into_iter(),
					value: None,
				}),
				other => Err(DeError(format!(
					"expected a map for struct variant, got {:?}",
					other.kind()
				))),
			}
		}
	}
	impl VariantAccess {
		fn require_payload(self) -> DeResult<Value> {
			self.payload
				.ok_or_else(|| DeError("expected a payload for variant".into()))
		}
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

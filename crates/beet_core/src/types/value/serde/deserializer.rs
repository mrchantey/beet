//! Deserializes [`Value`] into arbitrary Serde data.
use crate::prelude::*;
use ::serde::de;
use ::serde::de::IntoDeserializer;
use alloc::string::String;
use alloc::string::ToString;

/// Why a [`Value`] could not be deserialized into the target type.
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

/// A [`::serde::Deserializer`] that reads a [`Value`] directly.
///
/// This avoids round-tripping through `serde_json::Value`, preserving the
/// signed/unsigned distinction.
pub struct ValueDeserializer {
	value: Value,
}
impl ValueDeserializer {
	/// Read from `value`.
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

	::serde::forward_to_deserialize_any! {
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

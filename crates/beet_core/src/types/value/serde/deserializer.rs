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
pub struct ValueDeserializer<'a> {
	value: Value,
	references: References<'a>,
}

/// Resolves the `u64` a string names, see
/// [`ValueDeserializer::with_references`].
type References<'a> = Option<&'a dyn Fn(&str) -> Option<u64>>;

impl ValueDeserializer<'static> {
	/// Read from `value`.
	pub fn new(value: Value) -> Self {
		Self {
			value,
			references: None,
		}
	}
}

impl<'a> ValueDeserializer<'a> {
	/// Read a string as the `u64` `references` resolves it to, wherever the
	/// target asks for a `u64`, at any depth.
	///
	/// How a document writes a reference by name: a scene body writes an entity
	/// reference as its target's key, and the `Entity` its component expects
	/// reads it here. A map key is never resolved, since a number keyed map
	/// already spells its keys as strings.
	pub fn with_references<'b>(
		self,
		references: &'b dyn Fn(&str) -> Option<u64>,
	) -> ValueDeserializer<'b> {
		ValueDeserializer {
			value: self.value,
			references: Some(references),
		}
	}

	/// A deserializer over `value` resolving references as this one does.
	fn nested(value: Value, references: References<'a>) -> Self {
		Self { value, references }
	}
}

impl<'de, 'a> de::Deserializer<'de> for ValueDeserializer<'a> {
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
				references: self.references,
			}),
			Value::Map(map) => visitor.visit_map(MapAccess {
				iter: map.into_iter(),
				value: None,
				references: self.references,
			}),
		}
	}

	fn deserialize_u64<V: de::Visitor<'de>>(
		self,
		visitor: V,
	) -> DeResult<V::Value> {
		let resolved = match (&self.value, self.references) {
			(Value::Str(name), Some(references)) => references(name.as_str()),
			_ => None,
		};
		match resolved {
			Some(number) => visitor.visit_u64(number),
			None => self.deserialize_any(visitor),
		}
	}

	/// A byte string reads as its bytes for a target asking for a sequence,
	/// ie a `Vec<u8>` or a reflected list, which has no byte string form.
	fn deserialize_seq<V: de::Visitor<'de>>(
		self,
		visitor: V,
	) -> DeResult<V::Value> {
		match self.value {
			Value::Bytes(bytes) => visitor.visit_seq(
				de::value::SeqDeserializer::<_, DeError>::new(
					bytes.into_iter(),
				),
			),
			_ => self.deserialize_any(visitor),
		}
	}

	/// A byte string reads as its bytes for a fixed length target, ie a
	/// `[u8; N]`, as [`deserialize_seq`](Self::deserialize_seq) does.
	fn deserialize_tuple<V: de::Visitor<'de>>(
		self,
		_len: usize,
		visitor: V,
	) -> DeResult<V::Value> {
		self.deserialize_seq(visitor)
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
		visitor.visit_enum(EnumAccess {
			variant,
			payload,
			references: self.references,
		})
	}

	::serde::forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u128 f32 f64 char str
		string bytes byte_buf unit unit_struct tuple_struct
		map struct identifier ignored_any
	}
}

struct SeqAccess<'a> {
	iter: alloc::vec::IntoIter<Value>,
	references: References<'a>,
}
impl<'de, 'a> de::SeqAccess<'de> for SeqAccess<'a> {
	type Error = DeError;
	fn next_element_seed<T: de::DeserializeSeed<'de>>(
		&mut self,
		seed: T,
	) -> DeResult<Option<T::Value>> {
		match self.iter.next() {
			Some(value) => seed
				.deserialize(ValueDeserializer::nested(value, self.references))
				.map(Some),
			None => Ok(None),
		}
	}
}

struct MapAccess<'a> {
	iter: <Map as IntoIterator>::IntoIter,
	value: Option<Value>,
	references: References<'a>,
}
impl<'de, 'a> de::MapAccess<'de> for MapAccess<'a> {
	type Error = DeError;
	fn next_key_seed<K: de::DeserializeSeed<'de>>(
		&mut self,
		seed: K,
	) -> DeResult<Option<K::Value>> {
		match self.iter.next() {
			Some((key, value)) => {
				self.value = Some(value);
				seed.deserialize(MapKeyDeserializer(key)).map(Some)
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
		seed.deserialize(ValueDeserializer::nested(value, self.references))
	}
}

/// A map key: a string, or the number or bool it spells when the target asks
/// for one, as a json object key does. A `Value` map keyed by file keys (`"0"`,
/// `"1"`) therefore reads back into a `u32`-keyed map.
struct MapKeyDeserializer(SmolStr);

/// Parse the key as `$ty` for each integer `deserialize_*`, visiting the
/// widened value so every serde integer visitor accepts it.
macro_rules! deserialize_parsed_key {
	($($method:ident => $ty:ty => $visit:ident),* $(,)?) => {
		$(
			fn $method<V: de::Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
				match self.0.parse::<$ty>() {
					Ok(number) => visitor.$visit(number.into()),
					Err(_) => visitor.visit_str(self.0.as_str()),
				}
			}
		)*
	};
}

impl<'de> de::Deserializer<'de> for MapKeyDeserializer {
	type Error = DeError;

	fn deserialize_any<V: de::Visitor<'de>>(
		self,
		visitor: V,
	) -> DeResult<V::Value> {
		visitor.visit_str(self.0.as_str())
	}

	deserialize_parsed_key! {
		deserialize_i8 => i8 => visit_i64,
		deserialize_i16 => i16 => visit_i64,
		deserialize_i32 => i32 => visit_i64,
		deserialize_i64 => i64 => visit_i64,
		deserialize_u8 => u8 => visit_u64,
		deserialize_u16 => u16 => visit_u64,
		deserialize_u32 => u32 => visit_u64,
		deserialize_u64 => u64 => visit_u64,
	}

	fn deserialize_bool<V: de::Visitor<'de>>(
		self,
		visitor: V,
	) -> DeResult<V::Value> {
		match self.0.parse::<bool>() {
			Ok(bool) => visitor.visit_bool(bool),
			Err(_) => visitor.visit_str(self.0.as_str()),
		}
	}

	fn deserialize_option<V: de::Visitor<'de>>(
		self,
		visitor: V,
	) -> DeResult<V::Value> {
		visitor.visit_some(self)
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
		visitor.visit_enum(self.0.as_str().into_deserializer())
	}

	::serde::forward_to_deserialize_any! {
		i128 u128 f32 f64 char str string bytes byte_buf unit unit_struct
		seq tuple tuple_struct map struct identifier ignored_any
	}
}

struct EnumAccess<'a> {
	variant: SmolStr,
	payload: Option<Value>,
	references: References<'a>,
}
impl<'de, 'a> de::EnumAccess<'de> for EnumAccess<'a> {
	type Error = DeError;
	type Variant = VariantAccess<'a>;
	fn variant_seed<V: de::DeserializeSeed<'de>>(
		self,
		seed: V,
	) -> DeResult<(V::Value, Self::Variant)> {
		let variant =
			seed.deserialize(self.variant.as_str().into_deserializer())?;
		Ok((variant, VariantAccess {
			payload: self.payload,
			references: self.references,
		}))
	}
}

struct VariantAccess<'a> {
	payload: Option<Value>,
	references: References<'a>,
}
impl<'de, 'a> de::VariantAccess<'de> for VariantAccess<'a> {
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
		let references = self.references;
		seed.deserialize(ValueDeserializer::nested(
			self.require_payload()?,
			references,
		))
	}
	fn tuple_variant<V: de::Visitor<'de>>(
		self,
		_len: usize,
		visitor: V,
	) -> DeResult<V::Value> {
		let references = self.references;
		match self.require_payload()? {
			Value::List(list) => visitor.visit_seq(SeqAccess {
				iter: list.into_iter(),
				references,
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
		let references = self.references;
		match self.require_payload()? {
			Value::Map(map) => visitor.visit_map(MapAccess {
				iter: map.into_iter(),
				value: None,
				references,
			}),
			other => Err(DeError(format!(
				"expected a map for struct variant, got {:?}",
				other.kind()
			))),
		}
	}
}
impl VariantAccess<'_> {
	fn require_payload(self) -> DeResult<Value> {
		self.payload
			.ok_or_else(|| DeError("expected a payload for variant".into()))
	}
}

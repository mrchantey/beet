//! Serializes arbitrary Serde data into [`Value`].
use crate::prelude::*;
use ::serde::Serialize;
use ::serde::ser;
use alloc::string::String;
use alloc::string::ToString;

/// Why a value could not be serialized into a [`Value`].
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

/// A [`::serde::Serializer`] that builds a [`Value`] directly.
///
/// This avoids round-tripping through `serde_json::Value`, whose number type
/// collapses the signed/unsigned distinction.
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
	fn serialize_u16(self, v: u16) -> SerResult { Ok(Value::Uint(v as u64)) }
	fn serialize_u32(self, v: u32) -> SerResult { Ok(Value::Uint(v as u64)) }
	fn serialize_u64(self, v: u64) -> SerResult { Ok(Value::Uint(v)) }
	fn serialize_f32(self, v: f32) -> SerResult { Ok(Value::Float(v as f64)) }
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
		self.next_key = Some(key_to_smolstr(key.serialize(ValueSerializer)?)?);
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

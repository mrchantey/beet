//! A reflection-based value type for structured documents.
//!
//! This module provides [`Value`], a dynamically-typed value that supports
//! bidirectional conversion with Rust types via bevy_reflect. Unlike serde_json,
//! this type implements [`Reflect`] and is designed to work seamlessly with
//! richer document formats like automerge.
//!
//! # Converting Types to Values
//!
//! Use [`Value::from_reflect`] to convert any reflected type to a [`Value`]:
//!
//! ```ignore
//! #[derive(Reflect, Default)]
//! struct Player {
//!     name: String,
//!     score: i64,
//! }
//!
//! let player = Player { name: "Alice".into(), score: 100 };
//! let value = Value::from_reflect(&player).unwrap();
//! ```
//!
//! # Converting Values to Types
//!
//! Use [`Value::into_reflect`] to convert a [`Value`] back to a concrete type:
//!
//! ```ignore
//! let player: Player = value.into_reflect().unwrap();
//! ```

use beet_core::prelude::*;
use bevy::reflect::DynamicList;
use bevy::reflect::DynamicStruct;
use bevy::reflect::DynamicTupleStruct;
use bevy::reflect::FromReflect;
use bevy::reflect::PartialReflect;
use bevy::reflect::ReflectRef;
use bevy::reflect::StructInfo;
use bevy::reflect::TupleStructInfo;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use std::any::TypeId;

/// A dynamically-typed value for structured documents.
///
/// Similar in spirit to `serde_json::Value` but designed for bevy_reflect
/// integration and compatibility with richer document formats like automerge.
#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub enum Value {
	/// A map of string keys to values.
	Map(HashMap<String, Value>),
	/// An ordered list of values.
	List(Vec<Value>),
	/// Raw binary data.
	Bytes(Vec<u8>),
	/// A UTF-8 string.
	String(String),
	/// A 64-bit floating point number.
	F64(f64),
	/// An unsigned 64-bit integer.
	U64(u64),
	/// A signed 64-bit integer.
	I64(i64),
	/// A boolean value.
	Bool(bool),
	/// The absence of a value.
	Null,
}


impl ToString for Value {
	fn to_string(&self) -> String {
		match self {
			Value::Null => "null".to_string(),
			Value::Bool(b) => b.to_string(),
			Value::I64(n) => n.to_string(),
			Value::U64(n) => n.to_string(),
			Value::F64(f) => f.to_string(),
			Value::String(s) => s.clone(),
			Value::Bytes(bytes) => {
				format!(
					"[{}]",
					bytes
						.iter()
						.map(|b| b.to_string())
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
			Value::List(list) => {
				format!(
					"[{}]",
					list.iter()
						.map(|v| v.to_string())
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
			Value::Map(map) => {
				let mut entries: Vec<_> = map.iter().collect();
				entries.sort_by_key(|(k, _)| *k);
				format!(
					"{{{}}}",
					entries
						.iter()
						.map(|(k, v)| format!("{}: {}", k, v.to_string()))
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
		}
	}
}

impl std::hash::Hash for Value {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Value::Map(map) => {
				// Hash map entries in a deterministic order
				let mut entries: Vec<_> = map.iter().collect();
				entries.sort_by_key(|(k, _)| *k);
				for (key, value) in entries {
					key.hash(state);
					value.hash(state);
				}
			}
			Value::List(list) => list.hash(state),
			Value::Bytes(bytes) => bytes.hash(state),
			Value::String(s) => s.hash(state),
			Value::F64(f) => f.to_bits().hash(state),
			Value::U64(n) => n.hash(state),
			Value::I64(n) => n.hash(state),
			Value::Bool(b) => b.hash(state),
			Value::Null => {}
		}
	}
}

impl Eq for Value {}

impl Default for Value {
	fn default() -> Self { Self::Null }
}

impl Value {
	/// Creates a new null value.
	pub fn null() -> Self { Self::Null }

	/// Creates a new map value.
	pub fn map() -> Self { Self::Map(HashMap::default()) }

	/// Creates a new list value.
	pub fn list() -> Self { Self::List(Vec::new()) }

	/// Returns `true` if this value is null.
	pub fn is_null(&self) -> bool { matches!(self, Self::Null) }

	/// Returns `true` if this value is a map.
	pub fn is_map(&self) -> bool { matches!(self, Self::Map(_)) }

	/// Returns `true` if this value is a list.
	pub fn is_list(&self) -> bool { matches!(self, Self::List(_)) }

	/// Returns this value as a map reference, if it is one.
	pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
		match self {
			Self::Map(map) => Some(map),
			_ => None,
		}
	}

	/// Returns this value as a mutable map reference, if it is one.
	pub fn as_map_mut(&mut self) -> Option<&mut HashMap<String, Value>> {
		match self {
			Self::Map(map) => Some(map),
			_ => None,
		}
	}

	/// Returns this value as a list reference, if it is one.
	pub fn as_list(&self) -> Option<&Vec<Value>> {
		match self {
			Self::List(list) => Some(list),
			_ => None,
		}
	}

	/// Returns this value as a mutable list reference, if it is one.
	pub fn as_list_mut(&mut self) -> Option<&mut Vec<Value>> {
		match self {
			Self::List(list) => Some(list),
			_ => None,
		}
	}

	/// Returns this value as a string reference, if it is one.
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Self::String(s) => Some(s),
			_ => None,
		}
	}

	/// Returns this value as an i64, if it is one.
	pub fn as_i64(&self) -> Option<i64> {
		match self {
			Self::I64(n) => Some(*n),
			Self::U64(n) => i64::try_from(*n).ok(),
			_ => None,
		}
	}

	/// Returns this value as a u64, if it is one.
	pub fn as_u64(&self) -> Option<u64> {
		match self {
			Self::U64(n) => Some(*n),
			Self::I64(n) => u64::try_from(*n).ok(),
			_ => None,
		}
	}

	/// Returns this value as an f64, if it is one.
	pub fn as_f64(&self) -> Option<f64> {
		match self {
			Self::F64(n) => Some(*n),
			Self::I64(n) => Some(*n as f64),
			Self::U64(n) => Some(*n as f64),
			_ => None,
		}
	}

	/// Returns this value as a bool, if it is one.
	pub fn as_bool(&self) -> Option<bool> {
		match self {
			Self::Bool(b) => Some(*b),
			_ => None,
		}
	}

	/// Returns this value as bytes, if it is one.
	pub fn as_bytes(&self) -> Option<&[u8]> {
		match self {
			Self::Bytes(bytes) => Some(bytes),
			_ => None,
		}
	}

	/// Inserts a key-value pair into this value if it's a map.
	///
	/// Returns the previous value if the key existed, or `None` if this is not a map.
	pub fn insert(
		&mut self,
		key: impl Into<String>,
		value: impl Into<Value>,
	) -> Option<Value> {
		match self {
			Self::Map(map) => map.insert(key.into(), value.into()),
			_ => None,
		}
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

	/// Pushes a value onto this list if it is one.
	pub fn push(&mut self, value: impl Into<Value>) {
		if let Self::List(list) = self {
			list.push(value.into());
		}
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

	/// Convert a reflected type into a [`Value`].
	///
	/// Walks the reflection tree and builds a corresponding [`Value`] structure.
	///
	/// # Errors
	///
	/// Returns an error if the reflected type contains unsupported variants.
	pub fn from_reflect(reflect: &dyn PartialReflect) -> Result<Self> {
		value_from_reflect(reflect)
	}

	/// Convert this [`Value`] into a concrete type using reflection.
	///
	/// # Errors
	///
	/// Returns an error if the value structure doesn't match the target type
	/// or if conversion fails.
	pub fn into_reflect<T>(&self) -> Result<T>
	where
		T: 'static + Send + Sync + FromReflect + Typed,
	{
		let type_info = T::type_info();
		let dynamic = build_dynamic_from_value(self, type_info)?;
		T::from_reflect(dynamic.as_partial_reflect()).ok_or_else(|| {
			bevyhow!("failed to convert Value to {}", type_info.type_path())
		})
	}
}

/// Convert a reflected value to a [`Value`].
fn value_from_reflect(reflect: &dyn PartialReflect) -> Result<Value> {
	// Handle primitives first by trying to downcast
	if let Some(val) = reflect.try_downcast_ref::<bool>() {
		return Ok(Value::Bool(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<String>() {
		return Ok(Value::String(val.clone()));
	}
	if let Some(val) = reflect.try_downcast_ref::<&str>() {
		return Ok(Value::String((*val).to_string()));
	}

	// Signed integers
	if let Some(val) = reflect.try_downcast_ref::<i8>() {
		return Ok(Value::I64(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i16>() {
		return Ok(Value::I64(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i32>() {
		return Ok(Value::I64(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i64>() {
		return Ok(Value::I64(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<i128>() {
		return Ok(Value::I64(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<isize>() {
		return Ok(Value::I64(*val as i64));
	}

	// Unsigned integers
	if let Some(val) = reflect.try_downcast_ref::<u8>() {
		return Ok(Value::U64(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u16>() {
		return Ok(Value::U64(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u32>() {
		return Ok(Value::U64(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u64>() {
		return Ok(Value::U64(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<u128>() {
		return Ok(Value::U64(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<usize>() {
		return Ok(Value::U64(*val as u64));
	}

	// Floats
	if let Some(val) = reflect.try_downcast_ref::<f32>() {
		return Ok(Value::F64(*val as f64));
	}
	if let Some(val) = reflect.try_downcast_ref::<f64>() {
		return Ok(Value::F64(*val));
	}

	// Bytes - check specifically for Vec<u8> before generic list handling
	if let Some(val) = reflect.try_downcast_ref::<Vec<u8>>() {
		return Ok(Value::Bytes(val.clone()));
	}

	// Handle complex types via reflection
	match reflect.reflect_ref() {
		ReflectRef::Struct(s) => {
			let mut map = HashMap::default();
			for idx in 0..s.field_len() {
				let field_name = s.name_at(idx).ok_or_else(|| {
					bevyhow!("struct field at index {} has no name", idx)
				})?;
				let field_value = s.field_at(idx).ok_or_else(|| {
					bevyhow!("struct field at index {} not found", idx)
				})?;
				map.insert(
					field_name.to_string(),
					value_from_reflect(field_value)?,
				);
			}
			Ok(Value::Map(map))
		}
		ReflectRef::TupleStruct(ts) => {
			// Single-field tuple structs (newtypes) unwrap to their inner value
			if ts.field_len() == 1 {
				let field = ts.field(0).ok_or_else(|| {
					bevyhow!("tuple struct field 0 not found")
				})?;
				value_from_reflect(field)
			} else {
				// Multi-field tuple structs become lists
				let mut list = Vec::with_capacity(ts.field_len());
				for idx in 0..ts.field_len() {
					let field = ts.field(idx).ok_or_else(|| {
						bevyhow!(
							"tuple struct field at index {} not found",
							idx
						)
					})?;
					list.push(value_from_reflect(field)?);
				}
				Ok(Value::List(list))
			}
		}
		ReflectRef::Tuple(t) => {
			let mut list = Vec::with_capacity(t.field_len());
			for idx in 0..t.field_len() {
				let field = t.field(idx).ok_or_else(|| {
					bevyhow!("tuple field at index {} not found", idx)
				})?;
				list.push(value_from_reflect(field)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::List(l) => {
			let mut list = Vec::with_capacity(l.len());
			for idx in 0..l.len() {
				let item = l.get(idx).ok_or_else(|| {
					bevyhow!("list item at index {} not found", idx)
				})?;
				list.push(value_from_reflect(item)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::Array(a) => {
			let mut list = Vec::with_capacity(a.len());
			for idx in 0..a.len() {
				let item = a.get(idx).ok_or_else(|| {
					bevyhow!("array item at index {} not found", idx)
				})?;
				list.push(value_from_reflect(item)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::Map(m) => {
			let mut map = HashMap::default();
			for (key, value) in m.iter() {
				// Keys must be strings
				let key_str = key
					.try_downcast_ref::<String>()
					.map(|s| s.clone())
					.or_else(|| {
						key.try_downcast_ref::<&str>().map(|s| s.to_string())
					})
					.ok_or_else(|| bevyhow!("map key must be a string"))?;
				map.insert(key_str, value_from_reflect(value)?);
			}
			Ok(Value::Map(map))
		}
		ReflectRef::Set(s) => {
			let mut list = Vec::with_capacity(s.len());
			for item in s.iter() {
				list.push(value_from_reflect(item)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::Enum(e) => {
			// Handle Option<T> specially
			let type_path = reflect
				.get_represented_type_info()
				.map(|info| info.type_path())
				.unwrap_or("");

			if type_path.starts_with("core::option::Option") {
				match e.variant_name() {
					"None" => return Ok(Value::Null),
					"Some" => {
						let field = e.field_at(0).ok_or_else(|| {
							bevyhow!("Option::Some has no field")
						})?;
						return value_from_reflect(field);
					}
					_ => {}
				}
			}

			// Generic enum handling: create a map with variant name and fields
			let variant_name = e.variant_name();
			let mut variant_map = HashMap::default();

			match e.variant_type() {
				bevy::reflect::VariantType::Unit => {
					// Unit variant: just the name
					return Ok(Value::String(variant_name.to_string()));
				}
				bevy::reflect::VariantType::Tuple => {
					// Tuple variant: list of fields
					let mut fields = Vec::with_capacity(e.field_len());
					for idx in 0..e.field_len() {
						let field = e.field_at(idx).ok_or_else(|| {
							bevyhow!(
								"enum tuple field at index {} not found",
								idx
							)
						})?;
						fields.push(value_from_reflect(field)?);
					}
					variant_map
						.insert(variant_name.to_string(), Value::List(fields));
				}
				bevy::reflect::VariantType::Struct => {
					// Struct variant: map of fields
					let mut fields = HashMap::default();
					for idx in 0..e.field_len() {
						let field_name = e.name_at(idx).ok_or_else(|| {
							bevyhow!(
								"enum struct field at index {} has no name",
								idx
							)
						})?;
						let field = e.field_at(idx).ok_or_else(|| {
							bevyhow!(
								"enum struct field at index {} not found",
								idx
							)
						})?;
						fields.insert(
							field_name.to_string(),
							value_from_reflect(field)?,
						);
					}
					variant_map
						.insert(variant_name.to_string(), Value::Map(fields));
				}
			}
			Ok(Value::Map(variant_map))
		}
		ReflectRef::Opaque(_) => {
			bevybail!(
				"cannot convert opaque type to Value: {:?}",
				reflect.reflect_kind()
			)
		}
	}
}

/// Build a dynamic reflected value from a [`Value`] and type info.
fn build_dynamic_from_value(
	value: &Value,
	type_info: &TypeInfo,
) -> Result<Box<dyn PartialReflect>> {
	match type_info {
		TypeInfo::Struct(info) => build_dynamic_struct(value, info),
		TypeInfo::TupleStruct(info) => build_dynamic_tuple_struct(value, info),
		TypeInfo::Tuple(info) => build_dynamic_tuple(value, info),
		TypeInfo::List(info) => build_dynamic_list(value, info),
		TypeInfo::Array(info) => build_dynamic_array(value, info),
		TypeInfo::Map(info) => build_dynamic_map(value, info),
		TypeInfo::Set(_) => {
			bevybail!(
				"Set types are not supported for Value -> Type conversion"
			)
		}
		TypeInfo::Enum(info) => build_dynamic_enum(value, info),
		TypeInfo::Opaque(info) => build_opaque_value(value, info.type_id()),
	}
}

/// Build a [`DynamicStruct`] from a [`Value::Map`].
fn build_dynamic_struct(
	value: &Value,
	info: &StructInfo,
) -> Result<Box<dyn PartialReflect>> {
	let map = value.as_map().ok_or_else(|| {
		bevyhow!("expected Map value for struct, found {:?}", value)
	})?;

	let mut dynamic = DynamicStruct::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("struct field at index {} not found", field_idx)
		})?;
		let field_name = field.name();
		let field_type_id = field.type_id();
		let field_type_info = field.type_info();

		let field_value = map.get(field_name);

		if let Some(built) = build_field_value(
			field_value,
			field_name,
			field_type_id,
			field_type_info,
		)? {
			dynamic.insert_boxed(field_name, built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a [`DynamicTupleStruct`] from a [`Value`].
fn build_dynamic_tuple_struct(
	value: &Value,
	info: &TupleStructInfo,
) -> Result<Box<dyn PartialReflect>> {
	let mut dynamic = DynamicTupleStruct::default();

	// Single-field tuple structs (newtypes) unwrap from their inner value
	if info.field_len() == 1 {
		let field = info.field_at(0).ok_or_else(|| {
			bevyhow!("tuple struct field at index 0 not found")
		})?;

		if let Some(built) = build_field_value(
			Some(value),
			"0",
			field.type_id(),
			field.type_info(),
		)? {
			dynamic.insert_boxed(built);
		}
	} else {
		// Multi-field tuple structs expect a list
		let list = value.as_list().ok_or_else(|| {
			bevyhow!("expected List value for multi-field tuple struct")
		})?;

		for field_idx in 0..info.field_len() {
			let field = info.field_at(field_idx).ok_or_else(|| {
				bevyhow!("tuple struct field at index {} not found", field_idx)
			})?;

			let field_value = list.get(field_idx);

			if let Some(built) = build_field_value(
				field_value,
				&field_idx.to_string(),
				field.type_id(),
				field.type_info(),
			)? {
				dynamic.insert_boxed(built);
			}
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic tuple from a [`Value::List`].
fn build_dynamic_tuple(
	value: &Value,
	info: &bevy::reflect::TupleInfo,
) -> Result<Box<dyn PartialReflect>> {
	let list = value.as_list().ok_or_else(|| {
		bevyhow!("expected List value for tuple, found {:?}", value)
	})?;

	let mut dynamic = bevy::reflect::DynamicTuple::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("tuple field at index {} not found", field_idx)
		})?;

		let field_value = list.get(field_idx);

		if let Some(built) = build_field_value(
			field_value,
			&field_idx.to_string(),
			field.type_id(),
			field.type_info(),
		)? {
			dynamic.insert_boxed(built);
		} else {
			bevybail!("tuple field {} is missing", field_idx);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic list from a [`Value::List`].
fn build_dynamic_list(
	value: &Value,
	info: &bevy::reflect::ListInfo,
) -> Result<Box<dyn PartialReflect>> {
	let list = value.as_list().ok_or_else(|| {
		bevyhow!("expected List value for list type, found {:?}", value)
	})?;

	let mut dynamic = DynamicList::default();
	let item_type_info = info.item_info();

	for item in list {
		if let Some(built) = build_field_value(
			Some(item),
			"item",
			info.item_ty().id(),
			item_type_info,
		)? {
			dynamic.push_box(built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic array from a [`Value::List`].
fn build_dynamic_array(
	value: &Value,
	info: &bevy::reflect::ArrayInfo,
) -> Result<Box<dyn PartialReflect>> {
	let list = value.as_list().ok_or_else(|| {
		bevyhow!("expected List value for array type, found {:?}", value)
	})?;

	// For arrays, we need to build a DynamicList and let reflection handle it
	let mut dynamic = DynamicList::default();
	let item_type_info = info.item_info();

	for item in list {
		if let Some(built) = build_field_value(
			Some(item),
			"item",
			info.item_ty().id(),
			item_type_info,
		)? {
			dynamic.push_box(built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic map from a [`Value::Map`].
fn build_dynamic_map(
	value: &Value,
	info: &bevy::reflect::MapInfo,
) -> Result<Box<dyn PartialReflect>> {
	use bevy::reflect::Map;

	let map = value.as_map().ok_or_else(|| {
		bevyhow!("expected Map value for map type, found {:?}", value)
	})?;

	let mut dynamic = bevy::reflect::DynamicMap::default();
	let value_type_info = info.value_info();

	for (key, val) in map {
		if let Some(built) = build_field_value(
			Some(val),
			key,
			info.value_ty().id(),
			value_type_info,
		)? {
			dynamic.insert_boxed(Box::new(key.clone()), built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic enum from a [`Value`].
fn build_dynamic_enum(
	value: &Value,
	info: &bevy::reflect::EnumInfo,
) -> Result<Box<dyn PartialReflect>> {
	// Handle Option<T> specially
	let type_path = info.type_path();
	if type_path.starts_with("core::option::Option") {
		if value.is_null() {
			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic.set_variant("None", bevy::reflect::DynamicVariant::Unit);
			return Ok(Box::new(dynamic));
		} else {
			// Get the Some variant info
			let some_variant = info
				.variant("Some")
				.ok_or_else(|| bevyhow!("Option type missing Some variant"))?;

			let field_info = match some_variant {
				bevy::reflect::VariantInfo::Tuple(tuple_info) => tuple_info
					.field_at(0)
					.ok_or_else(|| bevyhow!("Option::Some missing field 0"))?,
				_ => bevybail!("Option::Some is not a tuple variant"),
			};

			let inner_value = build_field_value(
				Some(value),
				"0",
				field_info.type_id(),
				field_info.type_info(),
			)?
			.ok_or_else(|| bevyhow!("failed to build Option inner value"))?;

			let mut tuple = bevy::reflect::DynamicTuple::default();
			tuple.insert_boxed(inner_value);

			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic.set_variant(
				"Some",
				bevy::reflect::DynamicVariant::Tuple(tuple),
			);
			return Ok(Box::new(dynamic));
		}
	}

	// Generic enum handling
	match value {
		Value::String(variant_name) => {
			// Unit variant
			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic
				.set_variant(variant_name, bevy::reflect::DynamicVariant::Unit);
			Ok(Box::new(dynamic))
		}
		Value::Map(map) => {
			// Should have exactly one entry: variant_name -> fields
			if map.len() != 1 {
				bevybail!(
					"expected single-entry map for enum variant, found {} entries",
					map.len()
				);
			}

			let (variant_name, fields) = map.iter().next().unwrap();

			let variant_info = info.variant(variant_name).ok_or_else(|| {
				bevyhow!("unknown enum variant: {}", variant_name)
			})?;

			let mut dynamic = bevy::reflect::DynamicEnum::default();

			match variant_info {
				bevy::reflect::VariantInfo::Unit(_) => {
					dynamic.set_variant(
						variant_name,
						bevy::reflect::DynamicVariant::Unit,
					);
				}
				bevy::reflect::VariantInfo::Tuple(tuple_info) => {
					let list = fields.as_list().ok_or_else(|| {
						bevyhow!("expected list for tuple variant fields")
					})?;

					let mut tuple = bevy::reflect::DynamicTuple::default();
					for (idx, field_info) in tuple_info.iter().enumerate() {
						let field_value = list.get(idx);
						if let Some(built) = build_field_value(
							field_value,
							&idx.to_string(),
							field_info.type_id(),
							field_info.type_info(),
						)? {
							tuple.insert_boxed(built);
						}
					}
					dynamic.set_variant(
						variant_name,
						bevy::reflect::DynamicVariant::Tuple(tuple),
					);
				}
				bevy::reflect::VariantInfo::Struct(struct_info) => {
					let field_map = fields.as_map().ok_or_else(|| {
						bevyhow!("expected map for struct variant fields")
					})?;

					let mut struct_variant =
						bevy::reflect::DynamicStruct::default();
					for field_info in struct_info.iter() {
						let field_value = field_map.get(field_info.name());
						if let Some(built) = build_field_value(
							field_value,
							field_info.name(),
							field_info.type_id(),
							field_info.type_info(),
						)? {
							struct_variant
								.insert_boxed(field_info.name(), built);
						}
					}
					dynamic.set_variant(
						variant_name,
						bevy::reflect::DynamicVariant::Struct(struct_variant),
					);
				}
			}

			Ok(Box::new(dynamic))
		}
		_ => {
			bevybail!(
				"expected String or Map value for enum, found {:?}",
				value
			)
		}
	}
}

/// Build an opaque (primitive) value.
fn build_opaque_value(
	value: &Value,
	type_id: TypeId,
) -> Result<Box<dyn PartialReflect>> {
	// Handle primitives
	if type_id == TypeId::of::<bool>() {
		let b = value
			.as_bool()
			.ok_or_else(|| bevyhow!("expected Bool value for bool field"))?;
		return Ok(Box::new(b));
	}

	if type_id == TypeId::of::<String>() {
		let s = value.as_str().ok_or_else(|| {
			bevyhow!(
				"expected String value for String field, found {:?}",
				value
			)
		})?;
		return Ok(Box::new(s.to_string()));
	}

	// Signed integers
	if type_id == TypeId::of::<i8>() {
		let n = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i8 field"))?;
		return Ok(Box::new(n as i8));
	}
	if type_id == TypeId::of::<i16>() {
		let n = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i16 field"))?;
		return Ok(Box::new(n as i16));
	}
	if type_id == TypeId::of::<i32>() {
		let n = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i32 field"))?;
		return Ok(Box::new(n as i32));
	}
	if type_id == TypeId::of::<i64>() {
		let n = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i64 field"))?;
		return Ok(Box::new(n));
	}
	if type_id == TypeId::of::<i128>() {
		let n = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i128 field"))?;
		return Ok(Box::new(n as i128));
	}
	if type_id == TypeId::of::<isize>() {
		let n = value.as_i64().ok_or_else(|| {
			bevyhow!("expected integer value for isize field")
		})?;
		return Ok(Box::new(n as isize));
	}

	// Unsigned integers
	if type_id == TypeId::of::<u8>() {
		let n = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u8 field"))?;
		return Ok(Box::new(n as u8));
	}
	if type_id == TypeId::of::<u16>() {
		let n = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u16 field"))?;
		return Ok(Box::new(n as u16));
	}
	if type_id == TypeId::of::<u32>() {
		let n = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u32 field"))?;
		return Ok(Box::new(n as u32));
	}
	if type_id == TypeId::of::<u64>() {
		let n = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u64 field"))?;
		return Ok(Box::new(n));
	}
	if type_id == TypeId::of::<u128>() {
		let n = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u128 field"))?;
		return Ok(Box::new(n as u128));
	}
	if type_id == TypeId::of::<usize>() {
		let n = value.as_u64().ok_or_else(|| {
			bevyhow!("expected integer value for usize field")
		})?;
		return Ok(Box::new(n as usize));
	}

	// Floats
	if type_id == TypeId::of::<f32>() {
		let n = value
			.as_f64()
			.ok_or_else(|| bevyhow!("expected float value for f32 field"))?;
		return Ok(Box::new(n as f32));
	}
	if type_id == TypeId::of::<f64>() {
		let n = value
			.as_f64()
			.ok_or_else(|| bevyhow!("expected float value for f64 field"))?;
		return Ok(Box::new(n));
	}

	// Bytes
	if type_id == TypeId::of::<Vec<u8>>() {
		let bytes = value.as_bytes().ok_or_else(|| {
			bevyhow!("expected Bytes value for Vec<u8> field")
		})?;
		return Ok(Box::new(bytes.to_vec()));
	}

	bevybail!("unsupported opaque type")
}

/// Build a field value from an optional [`Value`] reference.
///
/// Returns `None` if the field value is `None` and the field can use its default.
fn build_field_value(
	value: Option<&Value>,
	field_name: &str,
	field_type_id: TypeId,
	field_type_info: Option<&TypeInfo>,
) -> Result<Option<Box<dyn PartialReflect>>> {
	// If no value provided and no type info, we can't proceed
	let Some(value) = value else {
		return Ok(None);
	};

	// Handle Null as missing
	if value.is_null() {
		return Ok(None);
	}

	// Try opaque types first
	if let Ok(result) = build_opaque_value(value, field_type_id) {
		return Ok(Some(result));
	}

	// Use type info for complex types
	if let Some(type_info) = field_type_info {
		return build_dynamic_from_value(value, type_info).map(Some);
	}

	bevybail!(
		"cannot build field '{}': no type info and not a primitive",
		field_name
	)
}

// Conversion From implementations for convenience

impl From<bool> for Value {
	fn from(val: bool) -> Self { Self::Bool(val) }
}

impl From<String> for Value {
	fn from(val: String) -> Self { Self::String(val) }
}

impl From<&str> for Value {
	fn from(val: &str) -> Self { Self::String(val.to_string()) }
}

impl From<i8> for Value {
	fn from(val: i8) -> Self { Self::I64(val as i64) }
}

impl From<i16> for Value {
	fn from(val: i16) -> Self { Self::I64(val as i64) }
}

impl From<i32> for Value {
	fn from(val: i32) -> Self { Self::I64(val as i64) }
}

impl From<i64> for Value {
	fn from(val: i64) -> Self { Self::I64(val) }
}

impl From<u8> for Value {
	fn from(val: u8) -> Self { Self::U64(val as u64) }
}

impl From<u16> for Value {
	fn from(val: u16) -> Self { Self::U64(val as u64) }
}

impl From<u32> for Value {
	fn from(val: u32) -> Self { Self::U64(val as u64) }
}

impl From<u64> for Value {
	fn from(val: u64) -> Self { Self::U64(val) }
}

impl From<f32> for Value {
	fn from(val: f32) -> Self { Self::F64(val as f64) }
}

impl From<f64> for Value {
	fn from(val: f64) -> Self { Self::F64(val) }
}

impl From<Vec<u8>> for Value {
	fn from(val: Vec<u8>) -> Self { Self::Bytes(val) }
}

impl<T: Into<Value>> From<Option<T>> for Value {
	fn from(val: Option<T>) -> Self {
		match val {
			Some(v) => v.into(),
			None => Self::Null,
		}
	}
}

impl<V: Into<Value>> From<HashMap<String, V>> for Value {
	fn from(val: HashMap<String, V>) -> Self {
		Self::Map(val.into_iter().map(|(k, v)| (k, v.into())).collect())
	}
}

/// Creates a [`Value`] from a literal expression.
///
/// # Example
///
/// ```ignore
/// use beet_stack::prelude::*;
///
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
			let mut map = $crate::exports::HashMap::default();
			$(
				map.insert($key.to_string(), $crate::val!($value));
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
	fn value_null_default() { Value::default().xpect_eq(Value::Null); }

	#[test]
	fn value_is_methods() {
		Value::Null.is_null().xpect_true();
		Value::Map(HashMap::default()).is_map().xpect_true();
		Value::List(Vec::new()).is_list().xpect_true();
	}

	#[test]
	fn value_from_primitives() {
		Value::from(true).xpect_eq(Value::Bool(true));
		Value::from("hello").xpect_eq(Value::String("hello".into()));
		Value::from(42i64).xpect_eq(Value::I64(42));
		Value::from(42u64).xpect_eq(Value::U64(42));
		Value::from(3.14f64).xpect_eq(Value::F64(3.14));
	}

	#[test]
	fn value_map_operations() {
		let mut val = Value::map();
		val.insert("key", "value");
		val.get("key").unwrap().as_str().unwrap().xpect_eq("value");
	}

	#[test]
	fn value_list_operations() {
		let mut val = Value::list();
		val.push(1i64);
		val.push(2i64);
		val.get_index(0).unwrap().as_i64().unwrap().xpect_eq(1);
		val.get_index(1).unwrap().as_i64().unwrap().xpect_eq(2);
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct SimpleStruct {
		name: String,
		count: i64,
		active: bool,
	}

	#[test]
	fn from_reflect_simple_struct() {
		let original = SimpleStruct {
			name: "test".into(),
			count: 42,
			active: true,
		};

		let value = Value::from_reflect(&original).unwrap();

		let map = value.as_map().unwrap();
		map.get("name").unwrap().as_str().unwrap().xpect_eq("test");
		map.get("count").unwrap().as_i64().unwrap().xpect_eq(42);
		map.get("active").unwrap().as_bool().unwrap().xpect_true();
	}

	#[test]
	fn into_reflect_simple_struct() {
		let value = val!({
			"name": "test",
			"count": 42i64,
			"active": true
		});

		let result: SimpleStruct = value.into_reflect().unwrap();
		result.name.xpect_eq("test");
		result.count.xpect_eq(42);
		result.active.xpect_true();
	}

	#[test]
	fn roundtrip_simple_struct() {
		let original = SimpleStruct {
			name: "roundtrip".into(),
			count: 100,
			active: false,
		};

		let value = Value::from_reflect(&original).unwrap();
		let result: SimpleStruct = value.into_reflect().unwrap();

		result.xpect_eq(original);
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct NestedStruct {
		inner: SimpleStruct,
		label: String,
	}

	#[test]
	fn roundtrip_nested_struct() {
		let original = NestedStruct {
			inner: SimpleStruct {
				name: "inner".into(),
				count: 5,
				active: true,
			},
			label: "outer".into(),
		};

		let value = Value::from_reflect(&original).unwrap();
		let result: NestedStruct = value.into_reflect().unwrap();

		result.xpect_eq(original);
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithVec {
		items: Vec<i64>,
	}

	#[test]
	fn roundtrip_with_vec() {
		let original = WithVec {
			items: vec![1, 2, 3],
		};

		let value = Value::from_reflect(&original).unwrap();
		let result: WithVec = value.into_reflect().unwrap();

		result.xpect_eq(original);
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithOption {
		maybe: Option<String>,
	}

	#[test]
	fn roundtrip_option_some() {
		let original = WithOption {
			maybe: Some("present".into()),
		};

		let value = Value::from_reflect(&original).unwrap();
		let result: WithOption = value.into_reflect().unwrap();

		result.xpect_eq(original);
	}

	#[test]
	fn roundtrip_option_none() {
		let original = WithOption { maybe: None };

		let value = Value::from_reflect(&original).unwrap();
		let result: WithOption = value.into_reflect().unwrap();

		result.xpect_eq(original);
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
		val!("hello").xpect_eq(Value::String("hello".into()));
	}

	#[test]
	fn val_macro_number() { val!(42).xpect_eq(Value::I64(42)); }

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
		// Floats may have precision issues, so compare with tolerance
		((result.float_32 - original.float_32).abs() < 0.001).xpect_true();
		((result.float_64 - original.float_64).abs() < 0.001).xpect_true();
	}
}

//! A dynamically-typed value for structured documents and element nodes.
//!
//! [`Value`] serves dual purpose: as an element/text node value (XML text node
//! or attribute value) and as a structured document value with [`Map`] and
//! [`List`] support for bidirectional conversion with Rust types via bevy_reflect.
//!
//! # Converting Types to Values
//!
//! Use [`Value::from_reflect`] to convert any reflected type to a [`Value`]:
//!
//! ```rust,no_run
//! # use beet_node::prelude::*;
//! # use beet_core::prelude::*;
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
//! ```rust,no_run
//! # use beet_node::prelude::*;
//! # use beet_core::prelude::*;
//! # #[derive(Reflect, Default)]
//! # struct Player { name: String, score: i64 }
//! # let value = Value::Map(Default::default());
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
use std::borrow::Cow;


/// Used either as an element node (xml text node) or as an attribute value.
/// A [`Value`] added to the same entity as an [`Element`] should be rendered
/// as the first child.
///
/// Also supports structured document data via [`Map`](Value::Map) and
/// [`List`](Value::List) variants, with bidirectional bevy_reflect conversion.
#[derive(Debug, Default, Clone, PartialEq, Reflect, Component)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Value {
	#[default]
	Null,
	Bool(bool),
	Int(i64),
	Uint(u64),
	Float(Float),
	Bytes(Vec<u8>),
	Str(String),
	/// A map of string keys to values.
	Map(HashMap<String, Value>),
	/// An ordered list of values.
	List(Vec<Value>),
}

impl Eq for Value {}

impl std::hash::Hash for Value {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Value::Null => {}
			Value::Bool(val) => val.hash(state),
			Value::Int(val) => val.hash(state),
			Value::Uint(val) => val.hash(state),
			Value::Float(val) => val.hash(state),
			Value::Bytes(val) => val.hash(state),
			Value::Str(val) => val.hash(state),
			Value::Map(map) => {
				// deterministic sorted key hashing
				let mut entries: Vec<_> = map.iter().collect();
				entries.sort_by_key(|(key, _)| *key);
				for (key, value) in entries {
					key.hash(state);
					value.hash(state);
				}
			}
			Value::List(list) => list.hash(state),
		}
	}
}


impl Value {
	/// Creates a new value from anything convertible.
	pub fn new(value: impl Into<Self>) -> Self { value.into() }

	/// Creates a new null value.
	pub fn null() -> Self { Self::Null }

	/// Creates a new empty map value.
	pub fn map() -> Self { Self::Map(HashMap::default()) }

	/// Creates a new empty list value.
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
			Value::Float(val) => Some(val.0),
			Value::Int(val) => Some(*val as f64),
			Value::Uint(val) => Some(*val as f64),
			_ => None,
		}
	}

	/// Returns this value as a string reference, if it is one.
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Value::Str(val) => Some(val),
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

	/// Inserts a key-value pair into this value if it's a map.
	///
	/// Returns the previous value if the key existed, or `None` if not a map.
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

	/// Optimistically parse a string into the most specific [`Value`] variant.
	///
	/// Attempts trimmed conversions in order:
	/// bool → uint → int → float (<18 chars) → string fallback (untrimmed).
	pub fn parse_string(input: &str) -> Self {
		let trimmed = input.trim();
		if let Ok(val) = trimmed.parse::<bool>() {
			Value::new(val)
		} else if let Ok(val) = trimmed.parse::<u64>() {
			Value::new(val)
		} else if let Ok(val) = trimmed.parse::<i64>() {
			Value::new(val)
		} else if trimmed.len() < 18
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
		value_from_reflect(reflect)
	}

	/// Convert this [`Value`] into a concrete type using reflection.
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

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Null => write!(f, "null"),
			Value::Bool(val) => write!(f, "{}", val),
			Value::Int(val) => write!(f, "{}", val),
			Value::Uint(val) => write!(f, "{}", val),
			Value::Float(val) => write!(f, "{}", val.0),
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
				entries.sort_by_key(|(key, _)| *key);
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

/// A wrapper around f64 that implements Eq and Hash by comparing the bit
/// representation of the float.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	PartialOrd,
	Deref,
	DerefMut,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct Float(pub f64);

impl Eq for Float {}

impl std::hash::Hash for Float {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.to_bits().hash(state);
	}
}

impl Ord for Float {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.0
			.partial_cmp(&other.0)
			.unwrap_or(std::cmp::Ordering::Equal)
	}
}


impl From<f64> for Float {
	fn from(value: f64) -> Self { Float(value) }
}

impl From<f32> for Float {
	fn from(value: f32) -> Self { Float(value as f64) }
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
	fn from(value: f64) -> Self { Value::Float(Float(value)) }
}

impl From<f32> for Value {
	fn from(value: f32) -> Self { Value::Float(Float(value as f64)) }
}

impl From<Float> for Value {
	fn from(value: Float) -> Self { Value::Float(value) }
}

impl From<String> for Value {
	fn from(value: String) -> Self { Value::Str(value) }
}

impl From<&str> for Value {
	fn from(value: &str) -> Self { Value::Str(value.to_string()) }
}

impl<'a> From<Cow<'a, str>> for Value {
	fn from(value: Cow<'a, str>) -> Self { Value::Str(value.into_owned()) }
}

impl From<Vec<u8>> for Value {
	fn from(value: Vec<u8>) -> Self { Value::Bytes(value) }
}

impl From<&[u8]> for Value {
	fn from(value: &[u8]) -> Self { Value::Bytes(value.to_vec()) }
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
				.map(|(key, val)| (key, val.into()))
				.collect(),
		)
	}
}


// ── Reflection: Value ← PartialReflect ──────────────────────────────

/// Convert a reflected value to a [`Value`].
fn value_from_reflect(reflect: &dyn PartialReflect) -> Result<Value> {
	// Handle primitives first by trying to downcast
	if let Some(val) = reflect.try_downcast_ref::<bool>() {
		return Ok(Value::Bool(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<String>() {
		return Ok(Value::Str(val.clone()));
	}
	if let Some(val) = reflect.try_downcast_ref::<&str>() {
		return Ok(Value::Str((*val).to_string()));
	}

	// Signed integers
	if let Some(val) = reflect.try_downcast_ref::<i8>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i16>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i32>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i64>() {
		return Ok(Value::Int(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<i128>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<isize>() {
		return Ok(Value::Int(*val as i64));
	}

	// Unsigned integers
	if let Some(val) = reflect.try_downcast_ref::<u8>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u16>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u32>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u64>() {
		return Ok(Value::Uint(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<u128>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<usize>() {
		return Ok(Value::Uint(*val as u64));
	}

	// Floats
	if let Some(val) = reflect.try_downcast_ref::<f32>() {
		return Ok(Value::Float(Float(*val as f64)));
	}
	if let Some(val) = reflect.try_downcast_ref::<f64>() {
		return Ok(Value::Float(Float(*val)));
	}

	// Bytes — check specifically for Vec<u8> before generic list handling
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

			// Generic enum: create a map with variant name and fields
			let variant_name = e.variant_name();

			match e.variant_type() {
				bevy::reflect::VariantType::Unit => {
					Ok(Value::Str(variant_name.to_string()))
				}
				bevy::reflect::VariantType::Tuple => {
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
					let mut variant_map = HashMap::default();
					variant_map
						.insert(variant_name.to_string(), Value::List(fields));
					Ok(Value::Map(variant_map))
				}
				bevy::reflect::VariantType::Struct => {
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
					let mut variant_map = HashMap::default();
					variant_map
						.insert(variant_name.to_string(), Value::Map(fields));
					Ok(Value::Map(variant_map))
				}
			}
		}
		ReflectRef::Opaque(_) => {
			bevybail!(
				"cannot convert opaque type to Value: {:?}",
				reflect.reflect_kind()
			)
		}
		other => {
			bevybail!("unsupported reflect kind: {:?}", other.kind())
		}
	}
}


// ── Reflection: Value → PartialReflect ──────────────────────────────

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
			bevybail!("Set types are not supported for Value → Type conversion")
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
		Value::Str(variant_name) => {
			// Unit variant
			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic
				.set_variant(variant_name, bevy::reflect::DynamicVariant::Unit);
			Ok(Box::new(dynamic))
		}
		Value::Map(map) => {
			// Should have exactly one entry: variant_name → fields
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
			bevybail!("expected Str or Map value for enum, found {:?}", value)
		}
	}
}

/// Build an opaque (primitive) value.
fn build_opaque_value(
	value: &Value,
	type_id: TypeId,
) -> Result<Box<dyn PartialReflect>> {
	if type_id == TypeId::of::<bool>() {
		let val = value
			.as_bool()
			.ok_or_else(|| bevyhow!("expected Bool value for bool field"))?;
		return Ok(Box::new(val));
	}

	if type_id == TypeId::of::<String>() {
		let val = value.as_str().ok_or_else(|| {
			bevyhow!("expected Str value for String field, found {:?}", value)
		})?;
		return Ok(Box::new(val.to_string()));
	}

	// Signed integers
	if type_id == TypeId::of::<i8>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i8 field"))?;
		return Ok(Box::new(val as i8));
	}
	if type_id == TypeId::of::<i16>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i16 field"))?;
		return Ok(Box::new(val as i16));
	}
	if type_id == TypeId::of::<i32>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i32 field"))?;
		return Ok(Box::new(val as i32));
	}
	if type_id == TypeId::of::<i64>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i64 field"))?;
		return Ok(Box::new(val));
	}
	if type_id == TypeId::of::<i128>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i128 field"))?;
		return Ok(Box::new(val as i128));
	}
	if type_id == TypeId::of::<isize>() {
		let val = value.as_i64().ok_or_else(|| {
			bevyhow!("expected integer value for isize field")
		})?;
		return Ok(Box::new(val as isize));
	}

	// Unsigned integers
	if type_id == TypeId::of::<u8>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u8 field"))?;
		return Ok(Box::new(val as u8));
	}
	if type_id == TypeId::of::<u16>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u16 field"))?;
		return Ok(Box::new(val as u16));
	}
	if type_id == TypeId::of::<u32>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u32 field"))?;
		return Ok(Box::new(val as u32));
	}
	if type_id == TypeId::of::<u64>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u64 field"))?;
		return Ok(Box::new(val));
	}
	if type_id == TypeId::of::<u128>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u128 field"))?;
		return Ok(Box::new(val as u128));
	}
	if type_id == TypeId::of::<usize>() {
		let val = value.as_u64().ok_or_else(|| {
			bevyhow!("expected integer value for usize field")
		})?;
		return Ok(Box::new(val as usize));
	}

	// Floats
	if type_id == TypeId::of::<f32>() {
		let val = value
			.as_f64()
			.ok_or_else(|| bevyhow!("expected float value for f32 field"))?;
		return Ok(Box::new(val as f32));
	}
	if type_id == TypeId::of::<f64>() {
		let val = value
			.as_f64()
			.ok_or_else(|| bevyhow!("expected float value for f64 field"))?;
		return Ok(Box::new(val));
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
/// Returns `None` if the field value is `None` and the field can use its
/// default.
fn build_field_value(
	value: Option<&Value>,
	field_name: &str,
	field_type_id: TypeId,
	field_type_info: Option<&TypeInfo>,
) -> Result<Option<Box<dyn PartialReflect>>> {
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


// ── val! macro ──────────────────────────────────────────────────────

/// Creates a [`Value`] from a literal expression.
///
/// # Example
///
/// ```rust,no_run
/// # use beet_node::prelude::*;
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
		Value::parse_string("3.14").xpect_eq(Value::Float(Float(3.14)));
		Value::parse_string("-383.484").xpect_eq(Value::Float(Float(-383.484)));
		Value::parse_string("0.0").xpect_eq(Value::Float(Float(0.0)));
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
		Value::Map(HashMap::default()).is_map().xpect_true();
		Value::List(Vec::new()).is_list().xpect_true();
	}

	#[test]
	fn value_from_primitives() {
		Value::from(true).xpect_eq(Value::Bool(true));
		Value::from("hello").xpect_eq(Value::Str("hello".into()));
		Value::from(42i64).xpect_eq(Value::Int(42));
		Value::from(42u64).xpect_eq(Value::Uint(42));
		Value::from(3.14f64).xpect_eq(Value::Float(Float(3.14)));
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

	#[test]
	fn display_map() {
		let mut val = Value::map();
		val.insert("a", 1i64);
		val.insert("b", 2i64);
		val.to_string().xpect_eq("{a: 1, b: 2}");
	}

	#[test]
	fn display_list() {
		let mut val = Value::list();
		val.push(1i64);
		val.push(2i64);
		val.push(3i64);
		val.to_string().xpect_eq("[1, 2, 3]");
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
		let mut input = HashMap::default();
		input.insert("key".to_string(), 42i64);
		let value = Value::from(input);
		value.is_map().xpect_true();
		value.get("key").unwrap().as_i64().unwrap().xpect_eq(42);
	}
}

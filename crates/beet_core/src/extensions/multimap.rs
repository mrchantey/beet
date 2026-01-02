//! Generic multimap and reflection-based parsing into concrete types.
//!
//! This module provides [`MultiMap`], a generic map that stores multiple values per key,
//! and the [`MultiMapReflectExt`] trait which enables converting
//! `MultiMap<String, String>` data into concrete Rust types using bevy_reflect.
//!
//! # Supported Types for Reflection Parsing
//!
//! The target type must derive `Reflect`, `FromReflect`, and `Default`, plus include
//! the `#[reflect(Default)]` attribute. Field types can be:
//! - `bool` - parsed from "true"/"false" strings
//! - `String` - direct string value
//! - `Option<String>` - `None` if key is missing
//! - `Vec<String>` - all values for a key
//! - Numeric types (`i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `f32`, `f64`)
//! - Newtype wrappers (single-field structs/tuple structs) - transparent, use parent field name
//! - Nested multi-field structs/tuple structs - fields are flattened
//! - `Vec<NewType>` - vectors of newtype wrappers
//!
//! # Optional Fields and Defaults
//!
//! When a field is not present in the map, the parser will use the struct's `Default` value
//! for that field. This allows flexible parsing where only provided fields are populated.
//!
//! **Important**: Both the struct and any nested structs must:
//! 1. Derive `Default`
//! 2. Include `#[reflect(Default)]` attribute
//!
//! Without `#[reflect(Default)]`, bevy's `FromReflect` cannot construct the type when
//! fields are missing.
//!
//! To make a field required even when the struct has `Default`, use the `#[reflect(@RequiredField)]`
//! attribute. Missing required fields will cause parsing to fail with an error.
//!
//! # Example
//!
//! ```ignore
//! # use beet_net::prelude::*;
//! # use bevy::prelude::*;
//! #[derive(Debug, Reflect, Default)]
//! #[reflect(Default)]
//! struct QueryParams {
//!     name: String,              // uses Default if missing
//!     verbose: bool,             // uses Default (false) if missing
//!     tags: Vec<String>,         // uses Default (empty vec) if missing
//!     limit: Option<String>,     // None if missing
//! }
//!
//! let mut map = MultiMap::new();
//! map.insert("name".into(), "test".into());
//! map.insert("verbose".into(), "true".into());
//! map.insert("tags".into(), "a".into());
//! map.insert("tags".into(), "b".into());
//!
//! let params: QueryParams = map.parse().unwrap();
//! assert_eq!(params.name, "test");
//! assert!(params.verbose);
//! assert_eq!(params.tags, vec!["a", "b"]);
//! assert!(params.limit.is_none());
//! ```
//!
//! # Example with Required Fields
//!
//! ```ignore
//! #[derive(Debug, Reflect, Default)]
//! #[reflect(Default)]
//! struct Config {
//!     #[reflect(@RequiredField)]
//!     api_key: String,           // must be present, will error if missing
//!     timeout: u32,              // uses Default if missing
//! }
//! ```

use crate::prelude::*;
use bevy::reflect::DynamicStruct;
use bevy::reflect::DynamicTuple;
use bevy::reflect::DynamicTupleStruct;
use bevy::reflect::FromReflect;
use bevy::reflect::PartialReflect;
use bevy::reflect::StructInfo;
use bevy::reflect::TupleInfo;
use bevy::reflect::TupleStructInfo;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use bevy::reflect::attributes::CustomAttributes;
use std::any::TypeId;
use std::borrow::Borrow;
use std::hash::BuildHasher;
use std::hash::Hash;
use std::str::FromStr;

/// Marker attribute indicating a field is required during MultiMap parsing.
///
/// By default, when parsing a type with `#[reflect(Default)]`, missing fields
/// will use the struct's `Default` value. Apply this attribute to a field
/// to make it required instead - parsing will fail with an error if the
/// field is not present in the map.
///
/// # Example
///
/// ```ignore
/// #[derive(Reflect, Default)]
/// #[reflect(Default)]
/// struct Config {
///     #[reflect(@RequiredField)]
///     api_key: String,    // must be present, errors if missing
///     timeout: u32,       // optional, uses Default (0) if missing
/// }
/// ```
#[derive(Debug, Copy, Clone, Reflect)]
pub struct RequiredField;

/// A multimap that stores multiple values per key.
///
/// Unlike a standard `HashMap`, this allows multiple values to be associated
/// with the same key. Values are stored in insertion order per key.
#[derive(Debug, Clone)]
pub struct MultiMap<K, V, S = FixedHasher> {
	inner: HashMap<K, Vec<V>, S>,
}

/// Type alias for the common case of string keys and values.
pub type StringMultiMap = MultiMap<String, String>;

impl<K, V, S: Default> Default for MultiMap<K, V, S> {
	fn default() -> Self {
		Self {
			inner: HashMap::default(),
		}
	}
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq
	for MultiMap<K, V, S>
{
	fn eq(&self, other: &Self) -> bool { self.inner == other.inner }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for MultiMap<K, V, S> {}

impl<K, V, S> MultiMap<K, V, S>
where
	K: Eq + Hash,
	S: BuildHasher + Default,
{
	/// Create a new empty multimap.
	pub fn new() -> Self { Self::default() }

	/// Insert a key with no values.
	/// If the key already exists, this is a no-op.
	pub fn insert_key(&mut self, key: K) { self.inner.entry(key).or_default(); }

	/// Insert a value for a key.
	/// If the key already exists, the value is appended to the existing values.
	pub fn insert(&mut self, key: K, value: V) {
		self.inner.entry(key).or_default().push(value);
	}

	/// Get the first value for a key.
	pub fn get<Q>(&self, key: &Q) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.get(key).and_then(|values| values.first())
	}

	/// Get all values for a key.
	pub fn get_vec<Q>(&self, key: &Q) -> Option<&Vec<V>>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.get(key)
	}

	/// Get a mutable reference to all values for a key.
	pub fn get_vec_mut<Q>(&mut self, key: &Q) -> Option<&mut Vec<V>>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.get_mut(key)
	}

	/// Check if key exists.
	pub fn contains_key<Q>(&self, key: &Q) -> bool
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.contains_key(key)
	}

	/// Returns true if the multimap contains no keys.
	pub fn is_empty(&self) -> bool { self.inner.is_empty() }

	/// Returns the number of keys in the multimap.
	pub fn len(&self) -> usize { self.inner.len() }

	/// Iterate over all key-values pairs.
	pub fn iter_all(&self) -> impl Iterator<Item = (&K, &Vec<V>)> {
		self.inner.iter()
	}

	/// Iterate over all keys.
	pub fn keys(&self) -> impl Iterator<Item = &K> { self.inner.keys() }

	/// Remove a key and all its values.
	pub fn remove<Q>(&mut self, key: &Q) -> Option<Vec<V>>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.remove(key)
	}

	/// Clear all entries.
	pub fn clear(&mut self) { self.inner.clear(); }
}

/// Trait for parsing a `MultiMap<String, String>` into a concrete reflected type.
#[extend::ext(name=MultiMapReflectExt)]
pub impl MultiMap<String, String> {
	/// Parse the multimap into a concrete type `T`.
	///
	/// The type `T` must implement `Reflect`, `FromReflect`, `Default`, and `Typed`,
	/// and must have the `#[reflect(Default)]` attribute. Nested structs are flattened,
	/// meaning all field names must be unique across the entire type hierarchy.
	///
	/// # Optional Fields and Defaults
	///
	/// Fields not present in the map will use the struct's `Default` value. This enables
	/// flexible parsing where only provided fields need to be specified.
	///
	/// To make a field required, use `#[reflect(@RequiredField)]` - parsing will fail
	/// if that field is missing.
	///
	/// # Nested Structs
	///
	/// Nested structs must also derive `Default` and include `#[reflect(Default)]`.
	/// Their fields are flattened into the parent's namespace.
	///
	/// # Example
	///
	/// ```ignore
	/// #[derive(Reflect, Default)]
	/// #[reflect(Default)]
	/// struct Config {
	///     host: String,
	///     port: u16,
	/// }
	///
	/// let mut map = MultiMap::new();
	/// map.insert("host".into(), "localhost".into());
	/// // port is missing, will use Default (0)
	///
	/// let config: Config = map.parse_reflect().unwrap();
	/// assert_eq!(config.host, "localhost");
	/// assert_eq!(config.port, 0);
	/// ```
	fn parse_reflect<T>(&self) -> Result<T>
	where
		T: 'static + Send + Sync + FromReflect + Typed,
	{
		let type_info = T::type_info();
		let dynamic = build_dynamic_from_type_info(self, type_info, None)?;
		T::from_reflect(dynamic.as_partial_reflect()).ok_or_else(|| {
			bevyhow!(
				"failed to convert dynamic type to {}",
				type_info.type_path()
			)
		})
	}
}

/// Build a dynamic reflected value from type info and a multimap.
fn build_dynamic_from_type_info(
	map: &MultiMap<String, String>,
	type_info: &TypeInfo,
	field_prefix: Option<&str>,
) -> Result<Box<dyn PartialReflect>> {
	match type_info {
		TypeInfo::Struct(info) => build_dynamic_struct(map, info),
		TypeInfo::TupleStruct(info) => {
			build_dynamic_tuple_struct(map, info, field_prefix)
		}
		TypeInfo::Tuple(info) => build_dynamic_tuple(map, info, field_prefix),
		other => {
			bevybail!(
				"unsupported type kind for ReflectMultiMap: {}\nSupported types are Struct, TupleStruct and Tuple",
				other.kind()
			)
		}
	}
}

/// Build a DynamicStruct from a multimap using struct field info.
fn build_dynamic_struct(
	map: &MultiMap<String, String>,
	info: &StructInfo,
) -> Result<Box<dyn PartialReflect>> {
	let mut dynamic = DynamicStruct::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("field at index {} not found", field_idx)
		})?;
		let field_name = field.name();
		let field_type_id = field.type_id();
		let field_type_info = field.type_info();
		let field_custom_attributes = field.custom_attributes();

		if let Some(value) = build_field_value(
			map,
			field_name,
			field_type_id,
			field_type_info,
			field_custom_attributes,
		)? {
			dynamic.insert_boxed(field_name, value);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a DynamicTupleStruct from a multimap using tuple struct field info.
fn build_dynamic_tuple_struct(
	map: &MultiMap<String, String>,
	info: &TupleStructInfo,
	field_prefix: Option<&str>,
) -> Result<Box<dyn PartialReflect>> {
	// only single-field tuple structs (newtypes) are supported
	if info.field_len() != 1 {
		bevybail!("multi-field tuple structs not supported");
	}

	// field_prefix is required for tuple structs
	let Some(prefix) = field_prefix else {
		bevybail!("top level tuple structs not supported");
	};

	let mut dynamic = DynamicTupleStruct::default();

	let field = info
		.field_at(0)
		.ok_or_else(|| bevyhow!("tuple struct field at index 0 not found"))?;

	let field_type_id = field.type_id();
	let field_type_info = field.type_info();
	let field_custom_attributes = field.custom_attributes();

	let value = build_field_value(
		map,
		prefix,
		field_type_id,
		field_type_info,
		field_custom_attributes,
	)?
	.ok_or_else(|| {
		bevyhow!("missing required field '{}' for tuple struct", prefix)
	})?;
	dynamic.insert_boxed(value);

	Ok(Box::new(dynamic))
}

/// Build a DynamicTuple from a multimap using tuple field info.
fn build_dynamic_tuple(
	map: &MultiMap<String, String>,
	info: &TupleInfo,
	field_prefix: Option<&str>,
) -> Result<Box<dyn PartialReflect>> {
	let mut dynamic = DynamicTuple::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("tuple field at index {} not found", field_idx)
		})?;
		// tuple fields are accessed by index as string, unless we have a prefix
		let field_name = if let Some(prefix) = field_prefix {
			prefix.to_string()
		} else {
			field_idx.to_string()
		};
		let field_type_id = field.type_id();
		let field_type_info = field.type_info();
		let field_custom_attributes = field.custom_attributes();

		let value = build_field_value(
			map,
			&field_name,
			field_type_id,
			field_type_info,
			field_custom_attributes,
		)?
		.ok_or_else(|| {
			bevyhow!("missing required field '{}' for tuple", field_name)
		})?;
		dynamic.insert_boxed(value);
	}

	Ok(Box::new(dynamic))
}

/// Build a field value from the multimap based on the field's type.
///
/// Returns `None` if the field is not present in the map and is not marked
/// as required with `#[reflect(@RequiredField)]`. This allows the caller
/// to skip inserting the field, letting bevy_reflect use the Default value.
///
/// For nested structs, tuples, and complex types, always attempts to build
/// them (returning `Some`) since they're flattened and don't have a direct
/// map entry.
fn build_field_value(
	map: &MultiMap<String, String>,
	field_name: &str,
	field_type_id: TypeId,
	field_type_info: Option<&TypeInfo>,
	custom_attributes: &CustomAttributes,
) -> Result<Option<Box<dyn PartialReflect>>> {
	// bool/flag fields are a special case, we just need to
	// check presence of the key
	if field_type_id == TypeId::of::<bool>() {
		return parse_bool_field(map, field_name);
	};

	// Check for complex/nested types first (structs, tuples, lists)
	// These are flattened and don't have a direct entry in the map
	if let Some(type_info) = field_type_info {
		match type_info {
			TypeInfo::Struct(struct_info) => {
				return build_dynamic_struct(map, struct_info).map(Some);
			}
			TypeInfo::TupleStruct(tuple_struct_info) => {
				// single-field tuple structs (newtypes) use parent field name
				return build_dynamic_tuple_struct(
					map,
					tuple_struct_info,
					Some(field_name),
				)
				.map(Some);
			}
			TypeInfo::Tuple(_) => {
				// tuples are always flattened
				return build_dynamic_from_type_info(map, type_info, None)
					.map(Some);
			}
			TypeInfo::List(list_info) => {
				// Handle Vec of newtype wrappers
				if let Some(item_type_info) = list_info.item_info() {
					let is_newtype = match item_type_info {
						TypeInfo::TupleStruct(tuple_struct) => {
							tuple_struct.field_len() == 1
						}
						TypeInfo::Struct(struct_info) => {
							struct_info.field_len() == 1
						}
						_ => false,
					};

					if is_newtype {
						let map_item =
							if let Some(values) = map.get_vec(field_name) {
								values
							} else if custom_attributes
								.get::<RequiredField>()
								.is_some()
							{
								bevybail!(
									"missing required field '{}'",
									field_name
								);
							} else {
								return Ok(None);
							};

						return parse_vec_newtype_field(
							map_item,
							item_type_info,
						)
						.map(Some);
					}
				}
			}
			_ => {}
		}
	}

	// Now check if field exists in map for primitive types
	let map_item = if let Some(values) = map.get_vec(field_name) {
		values
	} else if custom_attributes.get::<RequiredField>().is_some() {
		bevybail!("missing required field '{}'", field_name);
	} else {
		// field not present and not required, return None to use default
		return Ok(None);
	};

	// Handle primitive/leaf types
	match field_type_id {
		id if id == TypeId::of::<String>() => {
			return parse_string_field(map_item);
		}
		id if id == TypeId::of::<Option<String>>() => {
			return Ok(Some(Box::new(parse_option_string_field(map_item))));
		}
		id if id == TypeId::of::<Vec<String>>() => {
			return Ok(Some(Box::new(parse_vec_string_field(map_item))));
		}
		id if id == TypeId::of::<i8>() => {
			return parse_number_field::<i8>(map_item, field_name);
		}
		id if id == TypeId::of::<i16>() => {
			return parse_number_field::<i16>(map_item, field_name);
		}
		id if id == TypeId::of::<i32>() => {
			return parse_number_field::<i32>(map_item, field_name);
		}
		id if id == TypeId::of::<i64>() => {
			return parse_number_field::<i64>(map_item, field_name);
		}
		id if id == TypeId::of::<i128>() => {
			return parse_number_field::<i128>(map_item, field_name);
		}
		id if id == TypeId::of::<isize>() => {
			return parse_number_field::<isize>(map_item, field_name);
		}
		id if id == TypeId::of::<u8>() => {
			return parse_number_field::<u8>(map_item, field_name);
		}
		id if id == TypeId::of::<u16>() => {
			return parse_number_field::<u16>(map_item, field_name);
		}
		id if id == TypeId::of::<u32>() => {
			return parse_number_field::<u32>(map_item, field_name);
		}
		id if id == TypeId::of::<u64>() => {
			return parse_number_field::<u64>(map_item, field_name);
		}
		id if id == TypeId::of::<u128>() => {
			return parse_number_field::<u128>(map_item, field_name);
		}
		id if id == TypeId::of::<usize>() => {
			return parse_number_field::<usize>(map_item, field_name);
		}
		id if id == TypeId::of::<f32>() => {
			return parse_number_field::<f32>(map_item, field_name);
		}
		id if id == TypeId::of::<f64>() => {
			return parse_number_field::<f64>(map_item, field_name);
		}
		_ => {}
	}

	bevybail!(
		"unsupported field type for '{}', expected bool, String, Option<String>, Vec<String>, numeric types, or nested struct",
		field_name
	)
}

/// Parse a number field from the multimap.
///
/// Returns `None` if the field has no values, allowing the caller to use
/// the struct's default value for this field.
fn parse_number_field<T: FromStr + PartialReflect>(
	values: &Vec<String>,
	field_name: &str,
) -> Result<Option<Box<dyn PartialReflect>>>
where
	T::Err: std::fmt::Display,
{
	let value_str = values.first();

	if value_str.is_none() {
		return Ok(None);
	}

	let parsed = value_str.unwrap().parse::<T>().map_err(|err| {
		bevyhow!(
			"invalid numeric value for field '{}': '{}' ({})",
			field_name,
			value_str.unwrap(),
			err
		)
	})?;

	Ok(Some(Box::new(parsed)))
}

/// Parse a bool field from the multimap.
///
/// Returns `None` if the key doesn't exist, allowing the struct's default
/// value to be used. If the key exists with no value, returns `true` (flag-style).
/// Supports: "true"/"false", "1"/"0", "yes"/"no", "on"/"off", or empty string.
fn parse_bool_field(
	map: &MultiMap<String, String>,
	field_name: &str,
) -> Result<Option<Box<dyn PartialReflect>>> {
	match map.get_vec(field_name) {
		Some(values) if values.is_empty() => Ok(Some(Box::new(true))), // key exists but no values (flag-style)
		Some(values) => match values[0].to_lowercase().as_str() {
			"true" | "1" | "yes" | "on" | "" => Ok(Some(Box::new(true))),
			"false" | "0" | "no" | "off" => Ok(Some(Box::new(false))),
			other => bevybail!(
				"invalid bool value for field '{}': '{}', expected true/false",
				field_name,
				other
			),
		},
		None => Ok(None), // key doesn't exist, return None to use default
	}
}

/// Parse a String field from the multimap.
///
/// Returns `None` if no values are present, allowing the struct's default
/// value to be used (typically an empty string).
fn parse_string_field(
	values: &Vec<String>,
) -> Result<Option<Box<dyn PartialReflect>>> {
	match values.first() {
		Some(value) => Ok(Some(Box::new(value.clone()))),
		None => Ok(None),
	}
}

/// Parse an optional String field from the multimap.
fn parse_option_string_field(values: &Vec<String>) -> Option<String> {
	values.first().cloned()
}

/// Parse a Vec<String> field from the multimap (all values for the key).
fn parse_vec_string_field(values: &Vec<String>) -> Vec<String> {
	values.clone()
}

/// Parse a Vec of newtype wrappers from the multimap.
fn parse_vec_newtype_field(
	values: &Vec<String>,
	item_type_info: &TypeInfo,
) -> Result<Box<dyn PartialReflect>> {
	use bevy::reflect::DynamicList;

	let mut dynamic_list = DynamicList::default();

	for value in values {
		// create a temporary map with the value
		let mut temp_map = MultiMap::<String, String>::default();
		temp_map.insert("0".to_string(), value.clone());

		// build the newtype wrapper using index "0"
		let item =
			build_dynamic_from_type_info(&temp_map, item_type_info, Some("0"))?;
		dynamic_list.push_box(item);
	}

	Ok(Box::new(dynamic_list))
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::Reflect;
	use sweet::prelude::*;

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct SimpleStruct {
		name: String,
		verbose: bool,
	}

	#[test]
	fn parses_simple_struct() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());
		map.insert("verbose".to_string(), "true".to_string());

		let result: SimpleStruct = map.parse_reflect().unwrap();
		result.name.xpect_eq("test".to_string());
		result.verbose.xpect_true();
	}

	#[test]
	fn parses_bool_variants() {
		// true variants
		for val in ["true", "1", "yes", "on"] {
			let mut map = MultiMap::new();
			map.insert("name".to_string(), "x".to_string());
			map.insert("verbose".to_string(), val.to_string());
			let result: SimpleStruct = map.parse_reflect().unwrap();
			result.verbose.xpect_true();
		}

		// false variants
		for val in ["false", "0", "no", "off"] {
			let mut map = MultiMap::new();
			map.insert("name".to_string(), "x".to_string());
			map.insert("verbose".to_string(), val.to_string());
			let result: SimpleStruct = map.parse_reflect().unwrap();
			result.verbose.xpect_false();
		}
	}

	#[test]
	fn missing_bool_defaults_false() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());

		let result: SimpleStruct = map.parse_reflect().unwrap();
		result.verbose.xpect_false();
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithOptional {
		required: String,
		optional: Option<String>,
	}

	#[test]
	fn parses_optional_present() {
		let mut map = MultiMap::new();
		map.insert("required".to_string(), "req".to_string());
		map.insert("optional".to_string(), "opt".to_string());

		let result: WithOptional = map.parse_reflect().unwrap();
		result.required.xpect_eq("req".to_string());
		result.optional.xpect_eq(Some("opt".to_string()));
	}

	#[test]
	fn parses_optional_missing() {
		let mut map = MultiMap::new();
		map.insert("required".to_string(), "req".to_string());

		let result: WithOptional = map.parse_reflect().unwrap();
		result.required.xpect_eq("req".to_string());
		result.optional.xpect_eq(None);
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithVec {
		name: String,
		tags: Vec<String>,
	}

	#[test]
	fn parses_vec_multiple_values() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());
		map.insert("tags".to_string(), "a".to_string());
		map.insert("tags".to_string(), "b".to_string());
		map.insert("tags".to_string(), "c".to_string());

		let result: WithVec = map.parse_reflect().unwrap();
		result.name.xpect_eq("test".to_string());
		result.tags.xpect_eq(vec![
			"a".to_string(),
			"b".to_string(),
			"c".to_string(),
		]);
	}

	#[test]
	fn parses_vec_empty() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());

		let result: WithVec = map.parse_reflect().unwrap();
		result.tags.xpect_eq(Vec::<String>::new());
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Inner {
		inner_field: String,
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithNested {
		outer_field: String,
		nested: Inner,
	}

	#[test]
	fn parses_nested_struct_flattened() {
		let mut map = MultiMap::new();
		map.insert("outer_field".to_string(), "outer".to_string());
		map.insert("inner_field".to_string(), "inner".to_string());

		let result: WithNested = map.parse_reflect().unwrap();
		result.outer_field.xpect_eq("outer".to_string());
		result.nested.inner_field.xpect_eq("inner".to_string());
	}

	#[test]
	fn errors_on_missing_required_field() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct RequiredStruct {
			#[reflect(@RequiredField)]
			name: String,
		}

		let map = MultiMap::<String, String>::new();
		let result: Result<RequiredStruct> = map.parse_reflect();
		result.xpect_err();
	}

	#[test]
	fn errors_on_invalid_bool() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());
		map.insert("verbose".to_string(), "invalid".to_string());

		let result: Result<SimpleStruct> = map.parse_reflect();
		result.xpect_err();
	}

	#[test]
	fn empty_string_bool_is_true() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());
		map.insert("verbose".to_string(), "".to_string());

		let result: SimpleStruct = map.parse_reflect().unwrap();
		result.verbose.xpect_true();
	}

	#[test]
	fn empty_value_list_bool_is_true() {
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());
		map.insert_key("verbose".to_string());

		let result: SimpleStruct = map.parse_reflect().unwrap();
		result.verbose.xpect_true();
	}

	#[test]
	fn parses_tuple() {
		let mut map = MultiMap::new();
		map.insert("0".to_string(), "first".to_string());
		map.insert("1".to_string(), "second".to_string());

		let result: (String, String) = map.parse_reflect().unwrap();
		result.0.xpect_eq("first".to_string());
		result.1.xpect_eq("second".to_string());
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Level2 {
		deep_field: String,
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Level1 {
		mid_field: String,
		level2: Level2,
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Level0 {
		top_field: String,
		level1: Level1,
	}

	#[test]
	fn parses_deeply_nested_struct() {
		let mut map = MultiMap::new();
		map.insert("top_field".to_string(), "top".to_string());
		map.insert("mid_field".to_string(), "mid".to_string());
		map.insert("deep_field".to_string(), "deep".to_string());

		let result: Level0 = map.parse_reflect().unwrap();
		result.top_field.xpect_eq("top".to_string());
		result.level1.mid_field.xpect_eq("mid".to_string());
		result.level1.level2.deep_field.xpect_eq("deep".to_string());
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct AllFieldTypes {
		string_field: String,
		bool_field: bool,
		optional_field: Option<String>,
		vec_field: Vec<String>,
	}

	#[test]
	fn parses_all_field_types_together() {
		let mut map = MultiMap::new();
		map.insert("string_field".to_string(), "hello".to_string());
		map.insert("bool_field".to_string(), "true".to_string());
		map.insert("optional_field".to_string(), "present".to_string());
		map.insert("vec_field".to_string(), "a".to_string());
		map.insert("vec_field".to_string(), "b".to_string());

		let result: AllFieldTypes = map.parse_reflect().unwrap();
		result.string_field.xpect_eq("hello".to_string());
		result.bool_field.xpect_true();
		result.optional_field.xpect_eq(Some("present".to_string()));
		result
			.vec_field
			.xpect_eq(vec!["a".to_string(), "b".to_string()]);
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Foo(pub String);

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Bar(pub bool);

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct Bazz(pub Vec<String>);

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct ParamsWithNewtypes {
		foo: Foo,
		bar: Vec<Bar>,
		bazz: Bazz,
	}

	#[test]
	fn parses_newtype_tuple_struct_fields() {
		let mut map = MultiMap::new();
		map.insert("foo".to_string(), "hello".to_string());
		map.insert("bar".to_string(), "true".to_string());
		map.insert("bar".to_string(), "false".to_string());
		map.insert("bazz".to_string(), "a".to_string());
		map.insert("bazz".to_string(), "b".to_string());

		let result: ParamsWithNewtypes = map.parse_reflect().unwrap();
		result.foo.0.xpect_eq("hello".to_string());
		result.bar.len().xpect_eq(2);
		result.bar[0].0.xpect_true();
		result.bar[1].0.xpect_false();
		result
			.bazz
			.0
			.xpect_eq(vec!["a".to_string(), "b".to_string()]);
	}

	#[test]
	fn parses_exact_user_example() {
		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct UserFoo(pub String);

		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct UserBar(pub bool);

		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct UserBazz(pub Vec<String>);

		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct UserParams {
			foo: UserFoo,
			bar: Vec<UserBar>,
			bazz: UserBazz,
		}

		let mut map = MultiMap::new();
		map.insert("foo".to_string(), "test_value".to_string());
		map.insert("bar".to_string(), "true".to_string());
		map.insert("bar".to_string(), "false".to_string());
		map.insert("bar".to_string(), "true".to_string());
		map.insert("bazz".to_string(), "x".to_string());
		map.insert("bazz".to_string(), "y".to_string());
		map.insert("bazz".to_string(), "z".to_string());

		let result: UserParams = map.parse_reflect().unwrap();
		result.foo.0.xpect_eq("test_value".to_string());
		result.bar.len().xpect_eq(3);
		result.bar[0].0.xpect_true();
		result.bar[1].0.xpect_false();
		result.bar[2].0.xpect_true();
		result.bazz.0.xpect_eq(vec![
			"x".to_string(),
			"y".to_string(),
			"z".to_string(),
		]);
	}

	#[test]
	fn insert_key_creates_empty_entry() {
		let mut map = MultiMap::<String, String>::new();
		map.insert_key("flag".to_string());

		map.contains_key("flag").xpect_true();
		map.get_vec("flag").xpect_eq(Some(&Vec::<String>::new()));
	}

	#[test]
	fn insert_key_noop_if_exists() {
		let mut map = MultiMap::<String, String>::new();
		map.insert("key".to_string(), "value".to_string());
		map.insert_key("key".to_string());

		// Should not clear existing values
		map.get_vec("key")
			.xpect_eq(Some(&vec!["value".to_string()]));
	}

	#[test]
	fn multimap_basic_operations() {
		let mut map = MultiMap::<String, String>::new();
		map.insert("a".to_string(), "1".to_string());
		map.insert("a".to_string(), "2".to_string());
		map.insert("b".to_string(), "3".to_string());

		map.len().xpect_eq(2);
		map.is_empty().xpect_false();
		map.get("a").xpect_eq(Some(&"1".to_string()));
		map.get_vec("a")
			.xpect_eq(Some(&vec!["1".to_string(), "2".to_string()]));
		map.contains_key("a").xpect_true();
		map.contains_key("c").xpect_false();

		map.remove("a");
		map.len().xpect_eq(1);

		map.clear();
		map.is_empty().xpect_true();
	}

	#[test]
	fn parses_signed_integers() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct SignedInts {
			i8_field: i8,
			i16_field: i16,
			i32_field: i32,
			i64_field: i64,
			i128_field: i128,
			isize_field: isize,
		}

		let mut map = MultiMap::<String, String>::new();
		map.insert("i8_field".to_string(), "-42".to_string());
		map.insert("i16_field".to_string(), "-1000".to_string());
		map.insert("i32_field".to_string(), "-50000".to_string());
		map.insert("i64_field".to_string(), "-9223372036854775807".to_string());
		map.insert(
			"i128_field".to_string(),
			"-170141183460469231731687303715884105727".to_string(),
		);
		map.insert("isize_field".to_string(), "-12345".to_string());

		let result: SignedInts = map.parse_reflect().unwrap();
		result.i8_field.xpect_eq(-42);
		result.i16_field.xpect_eq(-1000);
		result.i32_field.xpect_eq(-50000);
		result.i64_field.xpect_eq(-9223372036854775807);
		result
			.i128_field
			.xpect_eq(-170141183460469231731687303715884105727);
		result.isize_field.xpect_eq(-12345);
	}

	#[test]
	fn parses_unsigned_integers() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct UnsignedInts {
			u8_field: u8,
			u16_field: u16,
			u32_field: u32,
			u64_field: u64,
			u128_field: u128,
			usize_field: usize,
		}

		let mut map = MultiMap::<String, String>::new();
		map.insert("u8_field".to_string(), "255".to_string());
		map.insert("u16_field".to_string(), "65535".to_string());
		map.insert("u32_field".to_string(), "4294967295".to_string());
		map.insert("u64_field".to_string(), "18446744073709551615".to_string());
		map.insert(
			"u128_field".to_string(),
			"340282366920938463463374607431768211455".to_string(),
		);
		map.insert("usize_field".to_string(), "99999".to_string());

		let result: UnsignedInts = map.parse_reflect().unwrap();
		result.u8_field.xpect_eq(255);
		result.u16_field.xpect_eq(65535);
		result.u32_field.xpect_eq(4294967295);
		result.u64_field.xpect_eq(18446744073709551615);
		result
			.u128_field
			.xpect_eq(340282366920938463463374607431768211455);
		result.usize_field.xpect_eq(99999);
	}

	#[test]
	fn parses_floats() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct Floats {
			f32_field: f32,
			f64_field: f64,
		}

		let mut map = MultiMap::<String, String>::new();
		map.insert("f32_field".to_string(), "3.14159".to_string());
		map.insert("f64_field".to_string(), "-2.718281828459045".to_string());

		let result: Floats = map.parse_reflect().unwrap();
		result.f32_field.xpect_close(3.14159);
		result.f64_field.xpect_close(-2.718281828459045);
	}

	#[test]
	fn errors_on_invalid_number() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct WithInt {
			value: i32,
		}

		let mut map = MultiMap::<String, String>::new();
		map.insert("value".to_string(), "not_a_number".to_string());

		let result: Result<WithInt> = map.parse_reflect();
		result.unwrap_err();
	}

	#[test]
	fn errors_on_missing_required_number() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct WithInt {
			#[reflect(@RequiredField)]
			value: i32,
		}

		let map = MultiMap::<String, String>::new();

		let result: Result<WithInt> = map.parse_reflect();
		result.unwrap_err();
	}

	#[test]
	fn parses_mixed_types_with_numbers() {
		#[derive(Debug, Reflect)]
		#[reflect(Default)]
		struct MixedTypes {
			name: String,
			count: u32,
			ratio: f64,
			enabled: bool,
		}

		impl Default for MixedTypes {
			fn default() -> Self {
				Self {
					name: default(),
					count: 7,
					ratio: default(),
					enabled: default(),
				}
			}
		}

		let mut map = MultiMap::new();
		map.insert("name".to_string(), "test".to_string());
		map.insert("count".to_string(), "42".to_string());
		map.insert("ratio".to_string(), "3.14".to_string());
		map.insert("enabled".to_string(), "true".to_string());

		let result: MixedTypes = map.parse_reflect().unwrap();
		result.name.xpect_eq("test".to_string());
		result.count.xpect_eq(42);
		result.ratio.xpect_eq(3.14);
		result.enabled.xpect_true();

		// test with missing fields, should use custom defaults
		let mut map = MultiMap::new();
		map.insert("name".to_string(), "partial".to_string());

		let result: MixedTypes = map.parse_reflect().unwrap();
		result.name.xpect_eq("partial".to_string());
		result.count.xpect_eq(7); // custom default
		result.ratio.xpect_eq(0.0); // default f64
		result.enabled.xpect_false(); // default bool
	}

	#[test]
	fn missing_optional_fields_use_defaults() {
		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct Config {
			host: String,
			port: u16,
			enabled: bool,
			tags: Vec<String>,
		}

		// provide only some fields
		let mut map = MultiMap::new();
		map.insert("host".to_string(), "localhost".to_string());

		let result: Config = map.parse_reflect().unwrap();
		result.host.xpect_eq("localhost".to_string());
		result.port.xpect_eq(0); // default u16
		result.enabled.xpect_false(); // default bool
		result.tags.xpect_eq(Vec::<String>::new()); // default vec
	}

	#[test]
	fn nested_struct_fields_use_defaults() {
		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct Database {
			host: String,
			port: u16,
		}

		#[derive(Debug, Reflect, Default, PartialEq)]
		#[reflect(Default)]
		struct AppConfig {
			app_name: String,
			database: Database,
		}

		// provide only top-level field and one nested field
		let mut map = MultiMap::new();
		map.insert("app_name".to_string(), "myapp".to_string());
		map.insert("host".to_string(), "db.example.com".to_string());
		// port is missing, should use default

		let result: AppConfig = map.parse_reflect().unwrap();
		result.app_name.xpect_eq("myapp".to_string());
		result.database.host.xpect_eq("db.example.com".to_string());
		result.database.port.xpect_eq(0); // default u16
	}

	#[test]
	fn required_field_with_defaults() {
		#[derive(Debug, Reflect, Default)]
		#[reflect(Default)]
		struct MixedRequirements {
			#[reflect(@RequiredField)]
			api_key: String,
			timeout: u32,
			retries: u32,
		}

		// missing required field should error
		let mut map = MultiMap::new();
		map.insert("timeout".to_string(), "30".to_string());

		let result: Result<MixedRequirements> = map.parse_reflect();
		result.unwrap_err();

		// with required field present, optional fields use defaults
		map.insert("api_key".to_string(), "secret123".to_string());

		let result: MixedRequirements = map.parse_reflect().unwrap();
		result.api_key.xpect_eq("secret123".to_string());
		result.timeout.xpect_eq(30);
		result.retries.xpect_eq(0); // default
	}

	#[test]
	fn custom_defaults_are_respected() {
		#[derive(Debug, Reflect, PartialEq)]
		#[reflect(Default)]
		struct CustomDefaults {
			name: String,
			count: u32,
			ratio: f64,
		}

		impl Default for CustomDefaults {
			fn default() -> Self {
				Self {
					name: "default_name".to_string(),
					count: 100,
					ratio: 0.5,
				}
			}
		}

		// empty map should use custom defaults
		let map = MultiMap::<String, String>::new();
		let result: CustomDefaults = map.parse_reflect().unwrap();
		result.name.xpect_eq("default_name".to_string());
		result.count.xpect_eq(100);
		result.ratio.xpect_eq(0.5);

		// partial map should mix provided values with defaults
		let mut map = MultiMap::new();
		map.insert("count".to_string(), "42".to_string());
		let result: CustomDefaults = map.parse_reflect().unwrap();
		result.name.xpect_eq("default_name".to_string());
		result.count.xpect_eq(42);
		result.ratio.xpect_eq(0.5);
	}
}

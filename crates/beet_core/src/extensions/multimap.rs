//! Generic multimap and reflection-based parsing into concrete types.
//!
//! This module provides [`MultiMap`], a generic map that stores multiple values per key,
//! and the [`MultiMapReflectExt`] trait which enables converting
//! `MultiMap<String, String>` data into concrete Rust types using bevy_reflect.
//!
//! # Supported Types for Reflection Parsing
//!
//! The target type must derive `Reflect` and `FromReflect`. Field types can be:
//! - `bool` - parsed from "true"/"false" strings
//! - `String` - direct string value
//! - `Option<String>` - `None` if key is missing
//! - `Vec<String>` - all values for a key
//! - Newtype wrappers (single-field structs/tuple structs) - transparent, use parent field name
//! - Nested multi-field structs/tuple structs - fields are flattened
//! - `Vec<NewType>` - vectors of newtype wrappers
//!
//! # Example
//!
//! ```ignore
//! # use beet_net::prelude::*;
//! # use bevy::prelude::*;
//! #[derive(Debug, Reflect, Default)]
//! struct QueryParams {
//!     name: String,
//!     verbose: bool,
//!     tags: Vec<String>,
//!     limit: Option<String>,
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
use std::any::TypeId;
use std::borrow::Borrow;
use std::hash::BuildHasher;
use std::hash::Hash;

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
	/// The type `T` must implement `Reflect`, `FromReflect`, and `Typed`.
	/// Nested structs are flattened, meaning all field names must be unique
	/// across the entire type hierarchy.
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

		let value =
			build_field_value(map, field_name, field_type_id, field_type_info)?;
		dynamic.insert_boxed(field_name, value);
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

	let value = build_field_value(map, prefix, field_type_id, field_type_info)?;
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

		let value = build_field_value(
			map,
			&field_name,
			field_type_id,
			field_type_info,
		)?;
		dynamic.insert_boxed(value);
	}

	Ok(Box::new(dynamic))
}

/// Build a field value from the multimap based on the field's type.
fn build_field_value(
	map: &MultiMap<String, String>,
	field_name: &str,
	field_type_id: TypeId,
	field_type_info: Option<&TypeInfo>,
) -> Result<Box<dyn PartialReflect>> {
	// Handle primitive/leaf types first
	if field_type_id == TypeId::of::<bool>() {
		let value = parse_bool_field(map, field_name)?;
		return Ok(Box::new(value));
	}

	if field_type_id == TypeId::of::<String>() {
		let value = parse_string_field(map, field_name)?;
		return Ok(Box::new(value));
	}

	if field_type_id == TypeId::of::<Option<String>>() {
		let value = parse_option_string_field(map, field_name);
		return Ok(Box::new(value));
	}

	if field_type_id == TypeId::of::<Vec<String>>() {
		let value = parse_vec_string_field(map, field_name);
		return Ok(Box::new(value));
	}

	// Handle Vec of newtype wrappers by checking if it's a List type
	if let Some(type_info) = field_type_info {
		if let TypeInfo::List(list_info) = type_info {
			// check if the item type is a single-field tuple struct or struct
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
					return parse_vec_newtype_field(
						map,
						field_name,
						item_type_info,
					);
				}
			}
		}
	}

	// Handle nested struct types
	if let Some(type_info) = field_type_info {
		match type_info {
			TypeInfo::Struct(struct_info) => {
				return build_dynamic_struct(map, struct_info);
			}
			TypeInfo::TupleStruct(tuple_struct_info) => {
				// single-field tuple structs (newtypes) use parent field name
				return build_dynamic_tuple_struct(
					map,
					tuple_struct_info,
					Some(field_name),
				);
			}
			TypeInfo::Tuple(_) => {
				// tuples are always flattened
				return build_dynamic_from_type_info(map, type_info, None);
			}
			_ => {}
		}
	}

	bevybail!(
		"unsupported field type for '{}', expected bool, String, Option<String>, Vec<String>, or nested struct",
		field_name
	)
}

/// Parse a bool field from the multimap.
fn parse_bool_field(
	map: &MultiMap<String, String>,
	field_name: &str,
) -> Result<bool> {
	match map.get_vec(field_name) {
		Some(values) if values.is_empty() => Ok(true), // key exists but no values (flag-style)
		Some(values) => match values[0].to_lowercase().as_str() {
			"true" | "1" | "yes" | "on" | "" => Ok(true),
			"false" | "0" | "no" | "off" => Ok(false),
			other => bevybail!(
				"invalid bool value for field '{}': '{}', expected true/false",
				field_name,
				other
			),
		},
		None => Ok(false), // key doesn't exist, default to false
	}
}

/// Parse a required String field from the multimap.
fn parse_string_field(
	map: &MultiMap<String, String>,
	field_name: &str,
) -> Result<String> {
	map.get(field_name)
		.cloned()
		.ok_or_else(|| bevyhow!("missing required field '{}'", field_name))
}

/// Parse an optional String field from the multimap.
fn parse_option_string_field(
	map: &MultiMap<String, String>,
	field_name: &str,
) -> Option<String> {
	map.get(field_name).cloned()
}

/// Parse a Vec<String> field from the multimap (all values for the key).
fn parse_vec_string_field(
	map: &MultiMap<String, String>,
	field_name: &str,
) -> Vec<String> {
	map.get_vec(field_name).cloned().unwrap_or_default()
}

/// Parse a Vec of newtype wrappers from the multimap.
fn parse_vec_newtype_field(
	map: &MultiMap<String, String>,
	field_name: &str,
	item_type_info: &TypeInfo,
) -> Result<Box<dyn PartialReflect>> {
	use bevy::reflect::DynamicList;

	let values = map.get_vec(field_name).map(|v| v.as_slice()).unwrap_or(&[]);
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
	struct Inner {
		inner_field: String,
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
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
		let map = MultiMap::<String, String>::new();
		let result: Result<SimpleStruct> = map.parse_reflect();
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
	struct Level2 {
		deep_field: String,
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	struct Level1 {
		mid_field: String,
		level2: Level2,
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
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
	struct Foo(pub String);

	#[derive(Debug, Reflect, Default, PartialEq)]
	struct Bar(pub bool);

	#[derive(Debug, Reflect, Default, PartialEq)]
	struct Bazz(pub Vec<String>);

	#[derive(Debug, Reflect, Default, PartialEq)]
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
		struct UserFoo(pub String);

		#[derive(Debug, Reflect, Default, PartialEq)]
		struct UserBar(pub bool);

		#[derive(Debug, Reflect, Default, PartialEq)]
		struct UserBazz(pub Vec<String>);

		#[derive(Debug, Reflect, Default, PartialEq)]
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
}

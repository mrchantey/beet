//! Reflection-based parsing of MultiMap into concrete types.
//!
//! This module provides the [`ReflectMultiMap`] trait which enables converting
//! `MultiMap<String, String>` data into concrete Rust types using bevy_reflect.
//!
//! # Supported Types
//!
//! The target type must derive `Reflect` and `FromReflect`. Field types can be:
//! - `bool` - parsed from "true"/"false" strings
//! - `String` - direct string value
//! - `Option<String>` - `None` if key is missing
//! - `Vec<String>` - all values for a key
//! - Nested structs/tuple structs (fields are flattened)
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

use beet_core::prelude::*;
use bevy::reflect::DynamicStruct;
use bevy::reflect::DynamicTuple;
use bevy::reflect::DynamicTupleStruct;
use bevy::reflect::FromReflect;
use bevy::reflect::PartialReflect;
use bevy::reflect::ReflectKind;
use bevy::reflect::StructInfo;
use bevy::reflect::TupleInfo;
use bevy::reflect::TupleStructInfo;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use std::any::TypeId;

type MultiMap = multimap::MultiMap<String, String, FixedHasher>;

/// Trait for parsing a `MultiMap` into a concrete reflected type.
pub trait ReflectMultiMap {
	/// Parse the multimap into a concrete type `T`.
	///
	/// The type `T` must implement `Reflect`, `FromReflect`, and `Typed`.
	/// Nested structs are flattened, meaning all field names must be unique
	/// across the entire type hierarchy.
	fn parse<T>(&self) -> Result<T>
	where
		T: 'static + Send + Sync + FromReflect + Typed;
}

impl ReflectMultiMap for MultiMap {
	fn parse<T>(&self) -> Result<T>
	where
		T: 'static + Send + Sync + FromReflect + Typed,
	{
		let type_info = T::type_info();
		let dynamic = build_dynamic_from_type_info(self, type_info)?;
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
	map: &MultiMap,
	type_info: &TypeInfo,
) -> Result<Box<dyn PartialReflect>> {
	match type_info {
		TypeInfo::Struct(info) => build_dynamic_struct(map, info),
		TypeInfo::TupleStruct(info) => build_dynamic_tuple_struct(map, info),
		TypeInfo::Tuple(info) => build_dynamic_tuple(map, info),
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
	map: &MultiMap,
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
	map: &MultiMap,
	info: &TupleStructInfo,
) -> Result<Box<dyn PartialReflect>> {
	let mut dynamic = DynamicTupleStruct::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("tuple struct field at index {} not found", field_idx)
		})?;
		// tuple struct fields are accessed by index as string
		let field_name = field_idx.to_string();
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

/// Build a DynamicTuple from a multimap using tuple field info.
fn build_dynamic_tuple(
	map: &MultiMap,
	info: &TupleInfo,
) -> Result<Box<dyn PartialReflect>> {
	let mut dynamic = DynamicTuple::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("tuple field at index {} not found", field_idx)
		})?;
		// tuple fields are accessed by index as string
		let field_name = field_idx.to_string();
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
	map: &MultiMap,
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

	// Handle nested struct types by flattening
	if let Some(type_info) = field_type_info {
		match type_info.kind() {
			ReflectKind::Struct
			| ReflectKind::TupleStruct
			| ReflectKind::Tuple => {
				return build_dynamic_from_type_info(map, type_info);
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
fn parse_bool_field(map: &MultiMap, field_name: &str) -> Result<bool> {
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
fn parse_string_field(map: &MultiMap, field_name: &str) -> Result<String> {
	map.get(field_name)
		.cloned()
		.ok_or_else(|| bevyhow!("missing required field '{}'", field_name))
}

/// Parse an optional String field from the multimap.
fn parse_option_string_field(
	map: &MultiMap,
	field_name: &str,
) -> Option<String> {
	map.get(field_name).cloned()
}

/// Parse a Vec<String> field from the multimap (all values for the key).
fn parse_vec_string_field(map: &MultiMap, field_name: &str) -> Vec<String> {
	map.get_vec(field_name)
		.map(|values| values.clone())
		.unwrap_or_default()
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::Reflect;
	use sweet::prelude::*;

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct SimpleStruct {
		name: String,
		verbose: bool,
	}

	#[test]
	fn parses_simple_struct() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("verbose".into(), "true".into());

		let result: SimpleStruct = map.parse().unwrap();
		result.name.xpect_eq("test".to_string());
		result.verbose.xpect_true();
	}

	#[test]
	fn parses_bool_variants() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("verbose".into(), "1".into());

		let result: SimpleStruct = map.parse().unwrap();
		result.verbose.xpect_true();

		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("verbose".into(), "yes".into());

		let result: SimpleStruct = map.parse().unwrap();
		result.verbose.xpect_true();

		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("verbose".into(), "false".into());

		let result: SimpleStruct = map.parse().unwrap();
		result.verbose.xpect_false();
	}

	#[test]
	fn missing_bool_defaults_false() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());

		let result: SimpleStruct = map.parse().unwrap();
		result.verbose.xpect_false();
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct WithOptional {
		required: String,
		optional: Option<String>,
	}

	#[test]
	fn parses_optional_present() {
		let mut map = MultiMap::default();
		map.insert("required".into(), "req".into());
		map.insert("optional".into(), "opt".into());

		let result: WithOptional = map.parse().unwrap();
		result.required.xpect_eq("req".to_string());
		result.optional.xpect_eq(Some("opt".to_string()));
	}

	#[test]
	fn parses_optional_missing() {
		let mut map = MultiMap::default();
		map.insert("required".into(), "req".into());

		let result: WithOptional = map.parse().unwrap();
		result.required.xpect_eq("req".to_string());
		result.optional.xpect_none();
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct WithVec {
		name: String,
		tags: Vec<String>,
	}

	#[test]
	fn parses_vec_multiple_values() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("tags".into(), "a".into());
		map.insert("tags".into(), "b".into());
		map.insert("tags".into(), "c".into());

		let result: WithVec = map.parse().unwrap();
		result.name.xpect_eq("test".to_string());
		result.tags.xpect_eq(vec![
			"a".to_string(),
			"b".to_string(),
			"c".to_string(),
		]);
	}

	#[test]
	fn parses_vec_empty() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());

		let result: WithVec = map.parse().unwrap();
		result.tags.xpect_empty();
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct Inner {
		inner_field: String,
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct WithNested {
		outer_field: String,
		nested: Inner,
	}

	#[test]
	fn parses_nested_struct_flattened() {
		let mut map = MultiMap::default();
		map.insert("outer_field".into(), "outer".into());
		map.insert("inner_field".into(), "inner".into());

		let result: WithNested = map.parse().unwrap();
		result.outer_field.xpect_eq("outer".to_string());
		result.nested.inner_field.xpect_eq("inner".to_string());
	}

	#[test]
	fn errors_on_missing_required_field() {
		let map = MultiMap::default();
		let result = map.parse::<SimpleStruct>();
		result.xpect_err();
	}

	#[test]
	fn errors_on_invalid_bool() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("verbose".into(), "maybe".into());

		let result = map.parse::<SimpleStruct>();
		result.xpect_err();
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct TupleStructWrapper(String, bool);

	#[test]
	fn parses_tuple_struct() {
		let mut map = MultiMap::default();
		map.insert("0".into(), "value".into());
		map.insert("1".into(), "true".into());

		let result: TupleStructWrapper = map.parse().unwrap();
		result.0.xpect_eq("value".to_string());
		result.1.xpect_true();
	}

	#[test]
	fn empty_string_bool_is_true() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert("verbose".into(), "".into());

		let result: SimpleStruct = map.parse().unwrap();
		result.verbose.xpect_true();
	}

	#[test]
	fn empty_value_list_bool_is_true() {
		let mut map = MultiMap::default();
		map.insert("name".into(), "test".into());
		map.insert_many("verbose".into(), vec![]);

		let result: SimpleStruct = map.parse().unwrap();
		result.verbose.xpect_true();
	}

	#[test]
	fn parses_tuple() {
		let mut map = MultiMap::default();
		map.insert("0".into(), "first".into());
		map.insert("1".into(), "true".into());

		let result: (String, bool) = map.parse().unwrap();
		result.0.xpect_eq("first".to_string());
		result.1.xpect_true();
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct Level2 {
		deep_field: String,
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct Level1 {
		mid_field: bool,
		level2: Level2,
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct Level0 {
		top_field: String,
		level1: Level1,
	}

	#[test]
	fn parses_deeply_nested_struct() {
		let mut map = MultiMap::default();
		map.insert("top_field".into(), "top".into());
		map.insert("mid_field".into(), "true".into());
		map.insert("deep_field".into(), "deep".into());

		let result: Level0 = map.parse().unwrap();
		result.top_field.xpect_eq("top".to_string());
		result.level1.mid_field.xpect_true();
		result.level1.level2.deep_field.xpect_eq("deep".to_string());
	}

	#[derive(Debug, Default, Reflect, PartialEq)]
	struct AllFieldTypes {
		string_field: String,
		bool_field: bool,
		optional_field: Option<String>,
		vec_field: Vec<String>,
	}

	#[test]
	fn parses_all_field_types_together() {
		let mut map = MultiMap::default();
		map.insert("string_field".into(), "hello".into());
		map.insert("bool_field".into(), "on".into());
		map.insert("optional_field".into(), "present".into());
		map.insert("vec_field".into(), "one".into());
		map.insert("vec_field".into(), "two".into());

		let result: AllFieldTypes = map.parse().unwrap();
		result.string_field.xpect_eq("hello".to_string());
		result.bool_field.xpect_true();
		result.optional_field.xpect_eq(Some("present".to_string()));
		result
			.vec_field
			.xpect_eq(vec!["one".to_string(), "two".to_string()]);
	}
}

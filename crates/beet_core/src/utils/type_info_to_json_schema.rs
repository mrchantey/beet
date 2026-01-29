//! Convert Bevy [`TypeInfo`] to JSON Schema.
//!
//! This module provides utilities for converting Bevy's reflection type information
//! into standard JSON Schema format, useful for API documentation, tool definitions,
//! and schema validation.
//!
//! # Example
//!
//! ```
//! use bevy::reflect::{Reflect, Typed};
//! use beet_core::utils::type_info_to_json_schema::type_info_to_json_schema;
//!
//! #[derive(Reflect)]
//! struct MyRequest {
//!     name: String,
//!     count: u32,
//!     enabled: Option<bool>,
//! }
//!
//! let schema = type_info_to_json_schema(MyRequest::type_info());
//! // Returns a JSON Schema object with properties, required fields, etc.
//! ```

use bevy::reflect::ArrayInfo;
use bevy::reflect::EnumInfo;
use bevy::reflect::ListInfo;
use bevy::reflect::MapInfo;
use bevy::reflect::NamedField;
use bevy::reflect::SetInfo;
use bevy::reflect::StructInfo;
use bevy::reflect::TupleInfo;
use bevy::reflect::TupleStructInfo;
use bevy::reflect::TypeInfo;
use bevy::reflect::UnnamedField;
use bevy::reflect::VariantInfo;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

/// Converts a Bevy [`TypeInfo`] to a JSON Schema [`Value`].
///
/// This function handles all Bevy reflect type variants:
/// - Structs → `object` with `properties`
/// - Enums → `oneOf` with variants (simple enums become string enums)
/// - Tuples/TupleStructs → `array` with `prefixItems`
/// - Lists/Arrays/Sets → `array` with `items`
/// - Maps → `object` with `additionalProperties`
/// - Opaque values → mapped to primitive JSON types
///
/// # Type Mapping
///
/// Rust types are mapped to JSON Schema types as follows:
/// - `String`, `&str`, `char` → `string`
/// - `i8`-`i128`, `isize` → `integer`
/// - `u8`-`u128`, `usize` → `integer`
/// - `f32`, `f64` → `number`
/// - `bool` → `boolean`
/// - `Vec<T>`, `[T; N]` → `array`
/// - Other types → `object`
pub fn type_info_to_json_schema(type_info: &TypeInfo) -> Value {
	let mut schema = match type_info {
		TypeInfo::Struct(info) => struct_to_schema(info),
		TypeInfo::TupleStruct(info) => tuple_struct_to_schema(info),
		TypeInfo::Tuple(info) => tuple_to_schema(info),
		TypeInfo::List(info) => list_to_schema(info),
		TypeInfo::Array(info) => array_to_schema(info),
		TypeInfo::Map(info) => map_to_schema(info),
		TypeInfo::Set(info) => set_to_schema(info),
		TypeInfo::Enum(info) => enum_to_schema(info),
		TypeInfo::Opaque(info) => type_path_to_schema(info.type_path()),
	};

	// Add type metadata
	if let Some(obj) = schema.as_object_mut() {
		let type_path = type_info.type_path();
		let short_name = type_info.type_path_table().short_path();

		// Only add metadata if not a primitive type
		if !is_primitive_type_path(type_path) {
			obj.insert(
				"title".to_string(),
				Value::String(short_name.to_string()),
			);
		}

		// Add docs if available and bevy_reflect_documentation feature is enabled
		#[cfg(feature = "bevy_reflect_documentation")]
		if let Some(docs) = type_info.docs() {
			obj.insert(
				"description".to_string(),
				Value::String(docs.to_string()),
			);
		}
	}

	schema
}

/// Converts a struct's [`TypeInfo`] to JSON Schema.
fn struct_to_schema(info: &StructInfo) -> Value {
	let mut properties = Map::new();
	let mut required = Vec::new();

	for field in info.iter() {
		let field_schema = named_field_to_schema(field);
		let field_name = field.name().to_string();

		if is_required_field(field.type_path()) {
			required.push(Value::String(field_name.clone()));
		}

		properties.insert(field_name, field_schema);
	}

	let mut schema = json!({
		"type": "object",
		"properties": properties,
		"additionalProperties": false,
	});

	if !required.is_empty() {
		schema
			.as_object_mut()
			.unwrap()
			.insert("required".to_string(), Value::Array(required));
	}

	schema
}

/// Converts a tuple struct's [`TypeInfo`] to JSON Schema.
fn tuple_struct_to_schema(info: &TupleStructInfo) -> Value {
	// Single-field tuple structs (newtypes) unwrap to their inner type
	if info.field_len() == 1 {
		let field = info.field_at(0).expect("tuple struct has 1 field");
		return type_path_to_schema(field.type_path());
	}

	let prefix_items: Vec<Value> = info
		.iter()
		.map(|field| unnamed_field_to_schema(field))
		.collect();

	json!({
		"type": "array",
		"prefixItems": prefix_items,
		"items": false,
	})
}

/// Converts a tuple's [`TypeInfo`] to JSON Schema.
fn tuple_to_schema(info: &TupleInfo) -> Value {
	// Unit type
	if info.field_len() == 0 {
		return json!({ "type": "null" });
	}

	let prefix_items: Vec<Value> = info
		.iter()
		.map(|field| unnamed_field_to_schema(field))
		.collect();

	json!({
		"type": "array",
		"prefixItems": prefix_items,
		"items": false,
	})
}

/// Converts a list's [`TypeInfo`] to JSON Schema.
fn list_to_schema(info: &ListInfo) -> Value {
	let item_schema = type_path_to_schema(info.item_ty().path());

	json!({
		"type": "array",
		"items": item_schema,
	})
}

/// Converts an array's [`TypeInfo`] to JSON Schema.
fn array_to_schema(info: &ArrayInfo) -> Value {
	let item_schema = type_path_to_schema(info.item_ty().path());

	json!({
		"type": "array",
		"items": item_schema,
		"minItems": info.capacity(),
		"maxItems": info.capacity(),
	})
}

/// Converts a map's [`TypeInfo`] to JSON Schema.
fn map_to_schema(info: &MapInfo) -> Value {
	let value_schema = type_path_to_schema(info.value_ty().path());

	json!({
		"type": "object",
		"additionalProperties": value_schema,
	})
}

/// Converts a set's [`TypeInfo`] to JSON Schema.
fn set_to_schema(info: &SetInfo) -> Value {
	let item_schema = type_path_to_schema(info.value_ty().path());

	json!({
		"type": "array",
		"items": item_schema,
		"uniqueItems": true,
	})
}

/// Converts an enum's [`TypeInfo`] to JSON Schema.
fn enum_to_schema(info: &EnumInfo) -> Value {
	// Check if this is a simple enum (all unit variants)
	let is_simple = info
		.iter()
		.all(|variant| matches!(variant, VariantInfo::Unit(_)));

	if is_simple {
		let variants: Vec<Value> = info
			.iter()
			.map(|variant| Value::String(variant.name().to_string()))
			.collect();

		return json!({
			"type": "string",
			"enum": variants,
		});
	}

	// Complex enum with data variants
	let one_of: Vec<Value> = info.iter().map(variant_to_schema).collect();

	json!({
		"oneOf": one_of,
	})
}

/// Converts an enum variant to JSON Schema.
fn variant_to_schema(variant: &VariantInfo) -> Value {
	match variant {
		VariantInfo::Unit(info) => {
			json!({
				"const": info.name(),
			})
		}
		VariantInfo::Tuple(info) => {
			if info.field_len() == 1 {
				// Single-field tuple variant: { "VariantName": <inner_type> }
				let field =
					info.field_at(0).expect("tuple variant has 1 field");
				let inner_schema = type_path_to_schema(field.type_path());
				let mut props = Map::new();
				props.insert(info.name().to_string(), inner_schema);

				json!({
					"type": "object",
					"properties": props,
					"required": [info.name()],
					"additionalProperties": false,
				})
			} else {
				// Multi-field tuple variant: { "VariantName": [<types>] }
				let prefix_items: Vec<Value> = info
					.iter()
					.map(|field| type_path_to_schema(field.type_path()))
					.collect();

				let inner_schema = json!({
					"type": "array",
					"prefixItems": prefix_items,
					"items": false,
				});

				let mut props = Map::new();
				props.insert(info.name().to_string(), inner_schema);

				json!({
					"type": "object",
					"properties": props,
					"required": [info.name()],
					"additionalProperties": false,
				})
			}
		}
		VariantInfo::Struct(info) => {
			// Struct variant: { "VariantName": { "field1": <type>, ... } }
			let mut properties = Map::new();
			let mut required = Vec::new();

			for field in info.iter() {
				let field_name = field.name().to_string();
				let field_schema = type_path_to_schema(field.type_path());

				if is_required_field(field.type_path()) {
					required.push(Value::String(field_name.clone()));
				}

				properties.insert(field_name, field_schema);
			}

			let mut inner_schema = json!({
				"type": "object",
				"properties": properties,
				"additionalProperties": false,
			});

			if !required.is_empty() {
				inner_schema
					.as_object_mut()
					.unwrap()
					.insert("required".to_string(), Value::Array(required));
			}

			let mut props = Map::new();
			props.insert(info.name().to_string(), inner_schema);

			json!({
				"type": "object",
				"properties": props,
				"required": [info.name()],
				"additionalProperties": false,
			})
		}
	}
}

/// Converts a named field to JSON Schema.
fn named_field_to_schema(field: &NamedField) -> Value {
	#[cfg(feature = "bevy_reflect_documentation")]
	{
		let mut schema = type_path_to_schema(field.type_path());

		// Add field docs if available
		if let Some(obj) = schema.as_object_mut() {
			if let Some(docs) = field.docs() {
				obj.insert(
					"description".to_string(),
					Value::String(docs.to_string()),
				);
			}
		}

		schema
	}

	#[cfg(not(feature = "bevy_reflect_documentation"))]
	type_path_to_schema(field.type_path())
}

/// Converts an unnamed field to JSON Schema.
fn unnamed_field_to_schema(field: &UnnamedField) -> Value {
	type_path_to_schema(field.type_path())
}

/// Maps a Rust type path to a JSON Schema type.
fn type_path_to_schema(type_path: &str) -> Value {
	// Handle Option<T> specially - extract inner type
	if let Some(inner) = extract_option_inner(type_path) {
		let inner_schema = type_path_to_schema(inner);
		// Option types can be null or the inner type
		return json!({
			"oneOf": [
				{ "type": "null" },
				inner_schema,
			]
		});
	}

	// Handle Vec<T>
	if let Some(inner) = extract_generic_inner(type_path, "Vec") {
		let inner_schema = type_path_to_schema(inner);
		return json!({
			"type": "array",
			"items": inner_schema,
		});
	}

	// Handle HashMap/BTreeMap
	if let Some(inner) = extract_map_value_type(type_path) {
		let value_schema = type_path_to_schema(inner);
		return json!({
			"type": "object",
			"additionalProperties": value_schema,
		});
	}

	// Handle HashSet/BTreeSet
	if let Some(inner) = extract_generic_inner(type_path, "HashSet")
		.or_else(|| extract_generic_inner(type_path, "BTreeSet"))
	{
		let inner_schema = type_path_to_schema(inner);
		return json!({
			"type": "array",
			"items": inner_schema,
			"uniqueItems": true,
		});
	}

	// Map primitive types
	let json_type = map_primitive_type(type_path);
	json!({ "type": json_type })
}

/// Maps a primitive Rust type path to a JSON Schema type string.
fn map_primitive_type(type_path: &str) -> &'static str {
	// Extract the short name for comparison
	let short_name = type_path
		.rsplit("::")
		.next()
		.unwrap_or(type_path)
		.trim_start_matches('&');

	match short_name {
		// String types
		"String" | "str" | "char" | "Cow<str>" => "string",

		// Unsigned integers
		"u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "integer",

		// Signed integers
		"i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "integer",

		// Floating point
		"f32" | "f64" => "number",

		// Boolean
		"bool" => "boolean",

		// Unit type
		"()" => "null",

		// Default to object for complex types
		_ => "object",
	}
}

/// Checks if a type path represents an Option type and extracts the inner type.
fn extract_option_inner(type_path: &str) -> Option<&str> {
	let path = type_path.trim();

	// Handle both core::option::Option<T> and Option<T>
	let inner = if path.starts_with("core::option::Option<") {
		path.strip_prefix("core::option::Option<")
	} else if path.starts_with("Option<") {
		path.strip_prefix("Option<")
	} else {
		None
	}?;

	// Remove trailing >
	inner.strip_suffix('>')
}

/// Extracts the inner type from a generic like `Vec<T>`.
fn extract_generic_inner<'a>(
	type_path: &'a str,
	generic_name: &str,
) -> Option<&'a str> {
	let path = type_path.trim();

	// Try full path first (alloc::vec::Vec<T>)
	let patterns = [
		format!("alloc::vec::{}<", generic_name),
		format!("std::vec::{}<", generic_name),
		format!("std::collections::{}<", generic_name),
		format!("alloc::collections::{}<", generic_name),
		format!("{}<", generic_name),
	];

	for pattern in &patterns {
		if let Some(rest) = path.strip_prefix(pattern.as_str()) {
			return rest.strip_suffix('>');
		}
	}

	None
}

/// Extracts the value type from a map type like `HashMap<K, V>`.
fn extract_map_value_type(type_path: &str) -> Option<&str> {
	let path = type_path.trim();

	let prefixes = [
		"std::collections::HashMap<",
		"std::collections::BTreeMap<",
		"bevy::platform::collections::HashMap<",
		"HashMap<",
		"BTreeMap<",
	];

	for prefix in &prefixes {
		if let Some(rest) = path.strip_prefix(*prefix) {
			let rest = rest.strip_suffix('>')?;
			// Find the comma separating K and V (accounting for nested generics)
			let mut depth = 0;
			for (idx, ch) in rest.char_indices() {
				match ch {
					'<' => depth += 1,
					'>' => depth -= 1,
					',' if depth == 0 => {
						return Some(rest[idx + 1..].trim());
					}
					_ => {}
				}
			}
		}
	}

	None
}

/// Checks if a field type path represents a required (non-Option) field.
fn is_required_field(type_path: &str) -> bool {
	!type_path.starts_with("core::option::Option<")
		&& !type_path.starts_with("Option<")
}

/// Checks if a type path represents a primitive JSON type.
fn is_primitive_type_path(type_path: &str) -> bool {
	let short_name = type_path.rsplit("::").next().unwrap_or(type_path);
	matches!(
		short_name,
		"String"
			| "str" | "char"
			| "u8" | "u16"
			| "u32" | "u64"
			| "u128" | "usize"
			| "i8" | "i16"
			| "i32" | "i64"
			| "i128" | "isize"
			| "f32" | "f64"
			| "bool" | "()"
	)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use bevy::reflect::Typed;

	#[derive(Reflect)]
	struct SimpleStruct {
		name: String,
		count: u32,
		enabled: bool,
	}

	#[derive(Reflect)]
	struct WithOptional {
		required_field: String,
		optional_field: Option<String>,
	}

	#[derive(Reflect)]
	struct WithNested {
		inner: SimpleStruct,
		value: i64,
	}

	#[derive(Reflect)]
	struct WithVec {
		items: Vec<String>,
		numbers: Vec<i32>,
	}

	#[derive(Reflect)]
	enum SimpleEnum {
		First,
		Second,
		Third,
	}

	#[derive(Reflect)]
	enum ComplexEnum {
		Unit,
		Tuple(String),
		Struct { x: f32, y: f32 },
	}

	#[derive(Reflect)]
	struct TupleStruct(String, i32);

	#[derive(Reflect)]
	struct NewtypeStruct(String);

	#[test]
	fn simple_struct_schema() {
		let schema = type_info_to_json_schema(SimpleStruct::type_info());
		let obj = schema.as_object().unwrap();

		obj.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("object");

		let properties = obj.get("properties").unwrap().as_object().unwrap();
		properties.contains_key("name").xpect_true();
		properties.contains_key("count").xpect_true();
		properties.contains_key("enabled").xpect_true();

		// Check field types
		properties
			.get("name")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");

		properties
			.get("count")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("integer");

		properties
			.get("enabled")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("boolean");

		// All fields should be required
		let required = obj.get("required").unwrap().as_array().unwrap();
		required.len().xpect_eq(3);
	}

	#[test]
	fn optional_fields_not_required() {
		let schema = type_info_to_json_schema(WithOptional::type_info());
		let obj = schema.as_object().unwrap();

		let required = obj.get("required").unwrap().as_array().unwrap();

		// Only required_field should be in required array
		required.len().xpect_eq(1);
		required[0].as_str().unwrap().xpect_eq("required_field");

		// Optional field should have oneOf with null
		let properties = obj.get("properties").unwrap().as_object().unwrap();
		let optional_schema = properties.get("optional_field").unwrap();
		optional_schema.get("oneOf").is_some().xpect_true();
	}

	#[test]
	fn vec_field_schema() {
		let schema = type_info_to_json_schema(WithVec::type_info());
		let obj = schema.as_object().unwrap();
		let properties = obj.get("properties").unwrap().as_object().unwrap();

		let items_schema = properties.get("items").unwrap();
		items_schema
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("array");
		items_schema
			.get("items")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");
	}

	#[test]
	fn simple_enum_schema() {
		let schema = type_info_to_json_schema(SimpleEnum::type_info());
		let obj = schema.as_object().unwrap();

		obj.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");

		let variants = obj.get("enum").unwrap().as_array().unwrap();
		variants.len().xpect_eq(3);

		let variant_names: Vec<&str> =
			variants.iter().map(|v| v.as_str().unwrap()).collect();
		variant_names.contains(&"First").xpect_true();
		variant_names.contains(&"Second").xpect_true();
		variant_names.contains(&"Third").xpect_true();
	}

	#[test]
	fn complex_enum_schema() {
		let schema = type_info_to_json_schema(ComplexEnum::type_info());
		let obj = schema.as_object().unwrap();

		let one_of = obj.get("oneOf").unwrap().as_array().unwrap();
		one_of.len().xpect_eq(3);
	}

	#[test]
	fn tuple_struct_schema() {
		let schema = type_info_to_json_schema(TupleStruct::type_info());
		let obj = schema.as_object().unwrap();

		obj.get("type").unwrap().as_str().unwrap().xpect_eq("array");

		let prefix_items = obj.get("prefixItems").unwrap().as_array().unwrap();
		prefix_items.len().xpect_eq(2);
	}

	#[test]
	fn newtype_struct_unwraps() {
		let schema = type_info_to_json_schema(NewtypeStruct::type_info());
		let obj = schema.as_object().unwrap();

		// Newtype should unwrap to inner type
		obj.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");
	}

	#[test]
	fn unit_type_schema() {
		let schema = type_info_to_json_schema(<()>::type_info());
		let obj = schema.as_object().unwrap();

		obj.get("type").unwrap().as_str().unwrap().xpect_eq("null");
	}

	#[test]
	fn extract_option_inner_works() {
		extract_option_inner("core::option::Option<String>")
			.unwrap()
			.xpect_eq("String");

		extract_option_inner("Option<i32>").unwrap().xpect_eq("i32");

		extract_option_inner("String").xpect_none();
	}

	#[test]
	fn is_required_field_works() {
		is_required_field("String").xpect_true();
		is_required_field("i32").xpect_true();
		is_required_field("core::option::Option<String>").xpect_false();
		is_required_field("Option<i32>").xpect_false();
	}

	#[test]
	fn primitive_type_mapping() {
		map_primitive_type("alloc::string::String").xpect_eq("string");
		map_primitive_type("String").xpect_eq("string");
		map_primitive_type("u32").xpect_eq("integer");
		map_primitive_type("i64").xpect_eq("integer");
		map_primitive_type("f64").xpect_eq("number");
		map_primitive_type("bool").xpect_eq("boolean");
		map_primitive_type("()").xpect_eq("null");
		map_primitive_type("MyCustomType").xpect_eq("object");
	}
}

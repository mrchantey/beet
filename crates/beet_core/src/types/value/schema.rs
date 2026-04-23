//! JSON Schema generation from Bevy [`TypeInfo`].
//!
//! Nested struct and enum types are recursively resolved and placed in a `$defs`
//! section at the root, referenced via `$ref`.
//!
//! # Example
//!
//! ```rust
//! use bevy::reflect::{Reflect, Typed};
//! use beet_core::prelude::*;
//!
//! #[derive(Reflect)]
//! struct MyRequest {
//!     name: String,
//!     count: u32,
//!     enabled: Option<bool>,
//! }
//!
//! // Returns a JSON Schema object with properties, required fields, etc.
//! let schema = Schema::new::<MyRequest>();
//! ```
use crate::prelude::*;
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
use bevy::reflect::Typed;
use bevy::reflect::VariantInfo;

/// A JSON Schema represented as a [`Value`].
#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Schema(Value);

impl Schema {
	/// Generate a schema for type `T`.
	pub fn new<T: Typed>() -> Self { Self::from_type_info(T::type_info()) }

	/// Generate a schema from [`TypeInfo`].
	pub fn from_type_info(type_info: &TypeInfo) -> Self {
		let mut ctx = SchemaCtx::new();
		let mut schema = build_schema(type_info, &mut ctx);
		if !ctx.defs.is_empty() {
			if let Ok(map) = schema.as_map_mut() {
				map.insert("$defs".into(), Value::Map(ctx.defs));
			}
		}
		Self(schema)
	}

	/// Wraps an existing [`Value`] as a schema.
	pub fn from_value(value: Value) -> Self { Self(value) }

	/// Returns the inner [`Value`].
	pub fn into_inner(self) -> Value { self.0 }

	/// Sanitizes this schema for OpenAI strict mode in place.
	///
	/// - Adds `"additionalProperties": false` to all object schemas
	/// - Converts `oneOf` to `anyOf`
	/// - Ensures all properties are in `required`
	pub fn sanitize_for_strict_mode(&mut self) -> &mut Self {
		sanitize_value_for_strict_mode(&mut self.0);
		self
	}
}

fn sanitize_value_for_strict_mode(value: &mut Value) {
	let Value::Map(obj) = value else { return };

	// recurse into nested schemas
	for key in ["properties", "items", "prefixItems", "$defs"] {
		if let Some(nested) = obj.get_mut(key) {
			match nested {
				Value::Map(map) => {
					for val in map.values_mut() {
						sanitize_value_for_strict_mode(val);
					}
				}
				Value::List(arr) => {
					for val in arr.iter_mut() {
						sanitize_value_for_strict_mode(val);
					}
				}
				_ => {}
			}
		}
	}

	// convert oneOf to anyOf (OpenAI strict mode forbids oneOf)
	if let Some(one_of) = obj.remove("oneOf") {
		obj.insert("anyOf".into(), one_of);
	}

	// recurse into anyOf / allOf variants
	for key in ["anyOf", "allOf"] {
		if let Some(Value::List(arr)) = obj.get_mut(key) {
			for val in arr.iter_mut() {
				sanitize_value_for_strict_mode(val);
			}
		}
	}

	// ensure all properties are required (OpenAI strict mode requirement)
	if let Some(Value::Map(props)) = obj.get("properties").cloned() {
		let all_keys: Vec<Value> =
			props.keys().map(|k| Value::Str(k.clone())).collect();
		if !all_keys.is_empty() {
			obj.insert("required".into(), Value::List(all_keys));
		}
	}

	// add additionalProperties: false to objects without it
	if obj.get("type").and_then(|t| t.as_str()) == Some("object") {
		if !obj.contains_key("additionalProperties") {
			obj.insert("additionalProperties".into(), Value::Bool(false));
		}
	}
}

/// Context that accumulates `$defs` entries while building a schema.
struct SchemaCtx {
	defs: Map,
	visited: HashSet<SmolStr>,
}

impl SchemaCtx {
	fn new() -> Self {
		Self {
			defs: Map::default(),
			visited: HashSet::default(),
		}
	}
}

/// Adds a type definition to `ctx.defs`.
fn add_type_def(name: &str, schema: Value, ctx: &mut SchemaCtx) {
	ctx.defs.insert(SmolStr::from(name), schema);
}

/// Builds a `{ "$ref": "#/$defs/<name>" }` value.
fn make_ref(name: &str) -> Value {
	let mut m = Map::default();
	m.insert(
		"$ref".into(),
		Value::str(format!("#/$defs/{}", name)),
	);
	Value::Map(m)
}

/// Builds the inline schema for a [`TypeInfo`], adding nested type definitions
/// to `ctx.defs`.
fn build_schema(type_info: &TypeInfo, ctx: &mut SchemaCtx) -> Value {
	let mut schema = match type_info {
		TypeInfo::Struct(info) => struct_to_schema(info, ctx),
		TypeInfo::TupleStruct(info) => tuple_struct_to_schema(info, ctx),
		TypeInfo::Tuple(info) => tuple_to_schema(info, ctx),
		TypeInfo::List(info) => list_to_schema(info, ctx),
		TypeInfo::Array(info) => array_to_schema(info, ctx),
		TypeInfo::Map(info) => map_to_schema(info, ctx),
		TypeInfo::Set(info) => set_to_schema(info, ctx),
		TypeInfo::Enum(info) => enum_to_schema(info, ctx),
		TypeInfo::Opaque(info) => type_path_to_schema(info.type_path()),
	};

	// Add type metadata
	if let Ok(map) = schema.as_map_mut() {
		let type_path = type_info.type_path();
		let short_name = type_info.type_path_table().short_path();

		if !is_primitive_type_path(type_path) {
			map.insert("title".into(), Value::str(short_name));
		}

		#[cfg(feature = "bevy_reflect_documentation")]
		if let Some(docs) = type_info.docs() {
			map.insert("description".into(), Value::str(docs));
		}
	}

	schema
}

/// Resolves a field/item type to its schema, adding complex types to `$defs`.
fn resolve_type(
	type_info: Option<&TypeInfo>,
	type_path: &str,
	ctx: &mut SchemaCtx,
) -> Value {
	let Some(info) = type_info else {
		return type_path_to_schema(type_path);
	};

	// Primitives are always inlined
	if is_primitive_type_path(type_path) {
		return type_path_to_schema(type_path);
	}

	match info {
		TypeInfo::Opaque(_) => type_path_to_schema(type_path),

		TypeInfo::List(list_info) => {
			let item_schema = resolve_type(
				list_info.item_info(),
				list_info.item_ty().path(),
				ctx,
			);
			let mut m = Map::default();
			m.insert("type".into(), Value::Str("array".into()));
			m.insert("items".into(), item_schema);
			Value::Map(m)
		}

		TypeInfo::Array(arr_info) => {
			let item_schema = resolve_type(
				arr_info.item_info(),
				arr_info.item_ty().path(),
				ctx,
			);
			let mut m = Map::default();
			m.insert("type".into(), Value::Str("array".into()));
			m.insert("items".into(), item_schema);
			m.insert(
				"minItems".into(),
				Value::Uint(arr_info.capacity() as u64),
			);
			m.insert(
				"maxItems".into(),
				Value::Uint(arr_info.capacity() as u64),
			);
			Value::Map(m)
		}

		TypeInfo::Map(map_info) => {
			let value_schema = resolve_type(
				map_info.value_info(),
				map_info.value_ty().path(),
				ctx,
			);
			let mut m = Map::default();
			m.insert("type".into(), Value::Str("object".into()));
			m.insert("additionalProperties".into(), value_schema);
			Value::Map(m)
		}

		TypeInfo::Set(set_info) => {
			let item_schema = type_path_to_schema(set_info.value_ty().path());
			let mut m = Map::default();
			m.insert("type".into(), Value::Str("array".into()));
			m.insert("items".into(), item_schema);
			m.insert("uniqueItems".into(), Value::Bool(true));
			Value::Map(m)
		}

		TypeInfo::Tuple(tuple_info) => {
			if tuple_info.field_len() == 0 {
				let mut m = Map::default();
				m.insert("type".into(), Value::Str("null".into()));
				return Value::Map(m);
			}
			let prefix_items: Vec<Value> = tuple_info
				.iter()
				.map(|field| {
					resolve_type(field.type_info(), field.type_path(), ctx)
				})
				.collect();
			let mut m = Map::default();
			m.insert("type".into(), Value::Str("array".into()));
			m.insert("prefixItems".into(), Value::List(prefix_items));
			m.insert("items".into(), Value::Bool(false));
			Value::Map(m)
		}

		TypeInfo::TupleStruct(ts_info) => {
			// Newtypes unwrap to their inner type
			if ts_info.field_len() == 1 {
				let field =
					ts_info.field_at(0).expect("tuple struct has 1 field");
				return resolve_type(field.type_info(), field.type_path(), ctx);
			}
			// Multi-field tuple structs → $defs
			let short_name = info.type_path_table().short_path();
			if !ctx.visited.contains(short_name) {
				ctx.visited.insert(SmolStr::from(short_name));
				let schema = build_schema(info, ctx);
				add_type_def(short_name, schema, ctx);
			}
			make_ref(short_name)
		}

		TypeInfo::Enum(enum_info) => {
			// Handle Option<T> specially
			if is_option_type(type_path) {
				if let Some(VariantInfo::Tuple(some_info)) =
					enum_info.variant("Some")
				{
					if let Some(field) = some_info.field_at(0) {
						let inner = resolve_type(
							field.type_info(),
							field.type_path(),
							ctx,
						);
						let mut null_m = Map::default();
						null_m.insert("type".into(), Value::Str("null".into()));
						let mut m = Map::default();
						m.insert(
							"oneOf".into(),
							Value::List(vec![Value::Map(null_m), inner]),
						);
						return Value::Map(m);
					}
				}
			}
			// Other enums → $defs
			let short_name = info.type_path_table().short_path();
			if !ctx.visited.contains(short_name) {
				ctx.visited.insert(SmolStr::from(short_name));
				let schema = build_schema(info, ctx);
				add_type_def(short_name, schema, ctx);
			}
			make_ref(short_name)
		}

		TypeInfo::Struct(_) => {
			let short_name = info.type_path_table().short_path();
			if !ctx.visited.contains(short_name) {
				ctx.visited.insert(SmolStr::from(short_name));
				let schema = build_schema(info, ctx);
				add_type_def(short_name, schema, ctx);
			}
			make_ref(short_name)
		}
	}
}

/// Converts a struct's [`TypeInfo`] to JSON Schema.
fn struct_to_schema(info: &StructInfo, ctx: &mut SchemaCtx) -> Value {
	let mut properties = Map::default();
	let mut required = Vec::new();

	for field in info.iter() {
		let field_schema = named_field_to_schema(field, ctx);
		let field_name = field.name();

		if is_required_field(field.type_path()) {
			required.push(Value::str(field_name));
		}
		properties.insert(SmolStr::from(field_name), field_schema);
	}

	let mut schema = Map::default();
	schema.insert("type".into(), Value::Str("object".into()));
	schema.insert("properties".into(), Value::Map(properties));
	schema.insert("additionalProperties".into(), Value::Bool(false));
	if !required.is_empty() {
		schema.insert("required".into(), Value::List(required));
	}
	Value::Map(schema)
}

/// Converts a tuple struct's [`TypeInfo`] to JSON Schema.
fn tuple_struct_to_schema(
	info: &TupleStructInfo,
	ctx: &mut SchemaCtx,
) -> Value {
	if info.field_len() == 1 {
		let field = info.field_at(0).expect("tuple struct has 1 field");
		return resolve_type(field.type_info(), field.type_path(), ctx);
	}

	let prefix_items: Vec<Value> = info
		.iter()
		.map(|field| resolve_type(field.type_info(), field.type_path(), ctx))
		.collect();

	let mut m = Map::default();
	m.insert("type".into(), Value::Str("array".into()));
	m.insert("prefixItems".into(), Value::List(prefix_items));
	m.insert("items".into(), Value::Bool(false));
	Value::Map(m)
}

/// Converts a tuple's [`TypeInfo`] to JSON Schema.
fn tuple_to_schema(info: &TupleInfo, ctx: &mut SchemaCtx) -> Value {
	if info.field_len() == 0 {
		let mut m = Map::default();
		m.insert("type".into(), Value::Str("null".into()));
		return Value::Map(m);
	}

	let prefix_items: Vec<Value> = info
		.iter()
		.map(|field| resolve_type(field.type_info(), field.type_path(), ctx))
		.collect();

	let mut m = Map::default();
	m.insert("type".into(), Value::Str("array".into()));
	m.insert("prefixItems".into(), Value::List(prefix_items));
	m.insert("items".into(), Value::Bool(false));
	Value::Map(m)
}

/// Converts a list's [`TypeInfo`] to JSON Schema.
fn list_to_schema(info: &ListInfo, ctx: &mut SchemaCtx) -> Value {
	let item_schema =
		resolve_type(info.item_info(), info.item_ty().path(), ctx);
	let mut m = Map::default();
	m.insert("type".into(), Value::Str("array".into()));
	m.insert("items".into(), item_schema);
	Value::Map(m)
}

/// Converts an array's [`TypeInfo`] to JSON Schema.
fn array_to_schema(info: &ArrayInfo, ctx: &mut SchemaCtx) -> Value {
	let item_schema =
		resolve_type(info.item_info(), info.item_ty().path(), ctx);
	let mut m = Map::default();
	m.insert("type".into(), Value::Str("array".into()));
	m.insert("items".into(), item_schema);
	m.insert("minItems".into(), Value::Uint(info.capacity() as u64));
	m.insert("maxItems".into(), Value::Uint(info.capacity() as u64));
	Value::Map(m)
}

/// Converts a map's [`TypeInfo`] to JSON Schema.
fn map_to_schema(info: &MapInfo, ctx: &mut SchemaCtx) -> Value {
	let value_schema =
		resolve_type(info.value_info(), info.value_ty().path(), ctx);
	let mut m = Map::default();
	m.insert("type".into(), Value::Str("object".into()));
	m.insert("additionalProperties".into(), value_schema);
	Value::Map(m)
}

/// Converts a set's [`TypeInfo`] to JSON Schema.
fn set_to_schema(info: &SetInfo, _ctx: &mut SchemaCtx) -> Value {
	let item_schema = type_path_to_schema(info.value_ty().path());
	let mut m = Map::default();
	m.insert("type".into(), Value::Str("array".into()));
	m.insert("items".into(), item_schema);
	m.insert("uniqueItems".into(), Value::Bool(true));
	Value::Map(m)
}

/// Converts an enum's [`TypeInfo`] to JSON Schema.
fn enum_to_schema(info: &EnumInfo, ctx: &mut SchemaCtx) -> Value {
	let is_simple = info
		.iter()
		.all(|variant| matches!(variant, VariantInfo::Unit(_)));

	if is_simple {
		let variants: Vec<Value> = info
			.iter()
			.map(|v| Value::str(v.name()))
			.collect();
		let mut m = Map::default();
		m.insert("type".into(), Value::Str("string".into()));
		m.insert("enum".into(), Value::List(variants));
		return Value::Map(m);
	}

	let one_of: Vec<Value> =
		info.iter().map(|v| variant_to_schema(v, ctx)).collect();
	let mut m = Map::default();
	m.insert("oneOf".into(), Value::List(one_of));
	Value::Map(m)
}

/// Converts an enum variant to JSON Schema.
fn variant_to_schema(variant: &VariantInfo, ctx: &mut SchemaCtx) -> Value {
	match variant {
		VariantInfo::Unit(info) => {
			let mut m = Map::default();
			m.insert("const".into(), Value::str(info.name()));
			Value::Map(m)
		}
		VariantInfo::Tuple(info) => {
			if info.field_len() == 1 {
				let field =
					info.field_at(0).expect("tuple variant has 1 field");
				let inner_schema =
					resolve_type(field.type_info(), field.type_path(), ctx);

				let mut props = Map::default();
				props.insert(SmolStr::from(info.name()), inner_schema);

				let mut m = Map::default();
				m.insert("type".into(), Value::Str("object".into()));
				m.insert("properties".into(), Value::Map(props));
				m.insert(
					"required".into(),
					Value::List(vec![Value::str(info.name())]),
				);
				m.insert("additionalProperties".into(), Value::Bool(false));
				Value::Map(m)
			} else {
				let prefix_items: Vec<Value> = info
					.iter()
					.map(|field| {
						resolve_type(field.type_info(), field.type_path(), ctx)
					})
					.collect();

				let mut inner = Map::default();
				inner.insert("type".into(), Value::Str("array".into()));
				inner.insert("prefixItems".into(), Value::List(prefix_items));
				inner.insert("items".into(), Value::Bool(false));

				let mut props = Map::default();
				props.insert(SmolStr::from(info.name()), Value::Map(inner));

				let mut m = Map::default();
				m.insert("type".into(), Value::Str("object".into()));
				m.insert("properties".into(), Value::Map(props));
				m.insert(
					"required".into(),
					Value::List(vec![Value::str(info.name())]),
				);
				m.insert("additionalProperties".into(), Value::Bool(false));
				Value::Map(m)
			}
		}
		VariantInfo::Struct(info) => {
			let mut properties = Map::default();
			let mut required = Vec::new();

			for field in info.iter() {
				let field_name = field.name();
				let field_schema =
					resolve_type(field.type_info(), field.type_path(), ctx);

				if is_required_field(field.type_path()) {
					required.push(Value::str(field_name));
				}
				properties.insert(SmolStr::from(field_name), field_schema);
			}

			let mut inner = Map::default();
			inner.insert("type".into(), Value::Str("object".into()));
			inner.insert("properties".into(), Value::Map(properties));
			inner.insert("additionalProperties".into(), Value::Bool(false));
			if !required.is_empty() {
				inner.insert("required".into(), Value::List(required));
			}

			let mut props = Map::default();
			props.insert(SmolStr::from(info.name()), Value::Map(inner));

			let mut m = Map::default();
			m.insert("type".into(), Value::Str("object".into()));
			m.insert("properties".into(), Value::Map(props));
			m.insert(
				"required".into(),
				Value::List(vec![Value::str(info.name())]),
			);
			m.insert("additionalProperties".into(), Value::Bool(false));
			Value::Map(m)
		}
	}
}

/// Converts a named field to JSON Schema, with optional doc comment support.
fn named_field_to_schema(field: &NamedField, ctx: &mut SchemaCtx) -> Value {
	#[cfg(feature = "bevy_reflect_documentation")]
	{
		let mut schema =
			resolve_type(field.type_info(), field.type_path(), ctx);

		if let Some(docs) = field.docs() {
			let description = Value::str(docs);
			// $ref cannot have sibling keywords in strict mode; wrap in anyOf
			if schema.get("$ref").is_some() {
				let mut m = Map::default();
				m.insert("anyOf".into(), Value::List(vec![schema]));
				m.insert("description".into(), description);
				return Value::Map(m);
			}
			if let Ok(obj) = schema.as_map_mut() {
				obj.insert("description".into(), description);
			}
		}

		schema
	}

	#[cfg(not(feature = "bevy_reflect_documentation"))]
	resolve_type(field.type_info(), field.type_path(), ctx)
}

/// Maps a Rust type path to a JSON Schema type.
fn type_path_to_schema(type_path: &str) -> Value {
	// Handle Option<T>
	if let Some(inner) = extract_option_inner(type_path) {
		let inner_schema = type_path_to_schema(inner);
		let mut null_m = Map::default();
		null_m.insert("type".into(), Value::Str("null".into()));
		let mut m = Map::default();
		m.insert(
			"oneOf".into(),
			Value::List(vec![Value::Map(null_m), inner_schema]),
		);
		return Value::Map(m);
	}

	// Handle Vec<T>
	if let Some(inner) = extract_generic_inner(type_path, "Vec") {
		let inner_schema = type_path_to_schema(inner);
		let mut m = Map::default();
		m.insert("type".into(), Value::Str("array".into()));
		m.insert("items".into(), inner_schema);
		return Value::Map(m);
	}

	// Handle HashMap/BTreeMap
	if let Some(inner) = extract_map_value_type(type_path) {
		let value_schema = type_path_to_schema(inner);
		let mut m = Map::default();
		m.insert("type".into(), Value::Str("object".into()));
		m.insert("additionalProperties".into(), value_schema);
		return Value::Map(m);
	}

	// Handle HashSet/BTreeSet
	if let Some(inner) = extract_generic_inner(type_path, "HashSet")
		.or_else(|| extract_generic_inner(type_path, "BTreeSet"))
	{
		let inner_schema = type_path_to_schema(inner);
		let mut m = Map::default();
		m.insert("type".into(), Value::Str("array".into()));
		m.insert("items".into(), inner_schema);
		m.insert("uniqueItems".into(), Value::Bool(true));
		return Value::Map(m);
	}

	// Map primitive types
	let json_type = map_primitive_type(type_path);
	let mut m = Map::default();
	m.insert("type".into(), Value::str(json_type));
	Value::Map(m)
}

/// Maps a primitive Rust type path to a JSON Schema type string.
fn map_primitive_type(type_path: &str) -> &'static str {
	let short_name = type_path
		.rsplit("::")
		.next()
		.unwrap_or(type_path)
		.trim_start_matches('&');

	match short_name {
		"String" | "str" | "char" | "Cow<str>" | "PathBuf" | "OsString" => {
			"string"
		}
		"u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "integer",
		"i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "integer",
		"f32" | "f64" => "number",
		"bool" => "boolean",
		"()" => "null",
		_ => "object",
	}
}

/// Checks if a type path represents an `Option` type.
fn is_option_type(type_path: &str) -> bool {
	type_path.starts_with("core::option::Option<")
		|| type_path.starts_with("Option<")
}

/// Extracts the inner type from `Option<T>`.
fn extract_option_inner(type_path: &str) -> Option<&str> {
	let path = type_path.trim();
	let inner = if path.starts_with("core::option::Option<") {
		path.strip_prefix("core::option::Option<")
	} else if path.starts_with("Option<") {
		path.strip_prefix("Option<")
	} else {
		None
	}?;
	inner.strip_suffix('>')
}

/// Extracts the inner type from a generic like `Vec<T>`.
fn extract_generic_inner<'a>(
	type_path: &'a str,
	generic_name: &str,
) -> Option<&'a str> {
	let path = type_path.trim();
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

/// Returns `true` if the field type represents a required (non-Option) field.
fn is_required_field(type_path: &str) -> bool {
	!type_path.starts_with("core::option::Option<")
		&& !type_path.starts_with("Option<")
}

/// Returns `true` if the type path represents a primitive JSON type.
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

	#[derive(Reflect)]
	struct ComplexStruct {
		simple_enum: SimpleEnum,
		complex_enum: Option<ComplexEnum>,
		field: bool,
	}

	#[test]
	fn simple_struct_schema() {
		let schema = Schema::new::<SimpleStruct>();

		schema
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("object");

		let props = schema.get("properties").unwrap().as_map().unwrap();
		props.contains_key("name").xpect_true();
		props.contains_key("count").xpect_true();
		props.contains_key("enabled").xpect_true();

		props
			.get("name")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");

		props
			.get("count")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("integer");

		props
			.get("enabled")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("boolean");

		let required = schema.get("required").unwrap().as_list().unwrap();
		required.len().xpect_eq(3);
	}

	#[test]
	fn complex_struct_schema() {
		let schema = Schema::new::<ComplexStruct>();

		let defs = schema.get("$defs").unwrap().as_map().unwrap();
		defs.contains_key("SimpleEnum").xpect_true();
		defs.contains_key("ComplexEnum").xpect_true();

		let simple_enum_def = defs.get("SimpleEnum").unwrap();
		simple_enum_def
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");
		let variants = simple_enum_def.get("enum").unwrap().as_list().unwrap();
		variants.len().xpect_eq(3);

		let complex_enum_def = defs.get("ComplexEnum").unwrap();
		let one_of = complex_enum_def.get("oneOf").unwrap().as_list().unwrap();
		one_of.len().xpect_eq(3);

		let props = schema.get("properties").unwrap().as_map().unwrap();

		props
			.get("simple_enum")
			.unwrap()
			.get("$ref")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("#/$defs/SimpleEnum");

		let ce_prop = props.get("complex_enum").unwrap();
		let ce_one_of = ce_prop.get("oneOf").unwrap().as_list().unwrap();
		ce_one_of.len().xpect_eq(2);
		ce_one_of[0]
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("null");
		ce_one_of[1]
			.get("$ref")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("#/$defs/ComplexEnum");

		props
			.get("field")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("boolean");
	}

	#[test]
	fn nested_struct_schema() {
		let schema = Schema::new::<WithNested>();

		let defs = schema.get("$defs").unwrap().as_map().unwrap();
		defs.contains_key("SimpleStruct").xpect_true();

		let simple_def = defs.get("SimpleStruct").unwrap();
		let simple_props =
			simple_def.get("properties").unwrap().as_map().unwrap();
		simple_props.contains_key("name").xpect_true();
		simple_props.contains_key("count").xpect_true();
		simple_props.contains_key("enabled").xpect_true();

		simple_props
			.get("name")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");

		let props = schema.get("properties").unwrap().as_map().unwrap();
		props
			.get("inner")
			.unwrap()
			.get("$ref")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("#/$defs/SimpleStruct");

		props
			.get("value")
			.unwrap()
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("integer");
	}

	#[test]
	fn optional_fields_not_required() {
		let schema = Schema::new::<WithOptional>();

		let required = schema.get("required").unwrap().as_list().unwrap();
		required.len().xpect_eq(1);
		required[0].as_str().unwrap().xpect_eq("required_field");

		let props = schema.get("properties").unwrap().as_map().unwrap();
		let optional_schema = props.get("optional_field").unwrap();
		optional_schema.get("oneOf").is_some().xpect_true();
	}

	#[test]
	fn vec_field_schema() {
		let schema = Schema::new::<WithVec>();
		let props = schema.get("properties").unwrap().as_map().unwrap();

		let items_schema = props.get("items").unwrap();
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
		let schema = Schema::new::<SimpleEnum>();

		schema
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");

		let variants = schema.get("enum").unwrap().as_list().unwrap();
		variants.len().xpect_eq(3);

		let variant_names: Vec<&str> =
			variants.iter().map(|v| v.as_str().unwrap()).collect();
		variant_names.contains(&"First").xpect_true();
		variant_names.contains(&"Second").xpect_true();
		variant_names.contains(&"Third").xpect_true();
	}

	#[test]
	fn complex_enum_schema() {
		let schema = Schema::new::<ComplexEnum>();
		let one_of = schema.get("oneOf").unwrap().as_list().unwrap();
		one_of.len().xpect_eq(3);
	}

	#[test]
	fn tuple_struct_schema() {
		let schema = Schema::new::<TupleStruct>();
		schema
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("array");
		let prefix_items =
			schema.get("prefixItems").unwrap().as_list().unwrap();
		prefix_items.len().xpect_eq(2);
	}

	#[test]
	fn newtype_struct_unwraps() {
		let schema = Schema::new::<NewtypeStruct>();
		schema
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("string");
	}

	#[test]
	fn unit_type_schema() {
		let schema = Schema::new::<()>();
		schema
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("null");
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

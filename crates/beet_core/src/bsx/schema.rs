//! Schema typing and runtime prop verification for BSX templates.
//!
//! A schema types a template's props (and a document's state). Two authoring
//! forms produce the same [`ValueSchema`]:
//!
//! - the Rust-type form: a `#[template]`'s typed signature, registered alongside
//!   its build bridge (see `beet_core`'s `ReflectTemplateSchema`);
//! - the JSON form: a `<script type="json" bx:schema>` block in a `.bsx`,
//!   parsed here into a [`ValueSchema`].
//!
//! Verification is always at runtime: [`verify_props`] gathers a tag's prop
//! attributes into a [`Value`], resolves the template's schema (substituting
//! composable [`ValueSchema::Reference`]s against the [`SchemaRegistry`]), and
//! validates, surfacing a missing required field or a type mismatch as a graceful
//! error that rides [`TemplateError`](beet_core::prelude::TemplateError) on the
//! root rather than panicking.

use super::ast::*;
use super::resolve::is_directive;
use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Verify the props supplied to `tag` against its registered prop schema.
///
/// Builds a [`Value::Map`] from the element's literal prop attributes, resolves
/// the template's [`ValueSchema`] against the world's [`SchemaRegistry`] (so a
/// composable [`ValueSchema::Reference`] is substituted), and validates. Returns
/// an `Err` describing the failures, which rides the root's `TemplateError`.
///
/// A tag with no registered schema, or props that are not plain values (an entity
/// reference, a field binding), is left unverified, since those resolve by other
/// means.
pub fn verify_props(
	el: &BsxElement,
	tag: &str,
	app_registry: &AppTypeRegistry,
	cx: &mut TemplateContext,
) -> Result<()> {
	let Some(schema) = template_schema_by_name(app_registry, tag) else {
		return Ok(());
	};
	verify_props_against(el, tag, &schema, cx)
}

/// Verify a tag's props against an explicit `schema`, the shared path for both a
/// Rust template (schema looked up via [`template_schema_by_name`]) and a BSX
/// template (schema from its `bx:schema` block).
///
/// Resolves composable [`ValueSchema::Reference`]s against the world's
/// [`SchemaRegistry`], then validates. A missing required field or type mismatch
/// is an `Err` that rides the root's `TemplateError`.
pub fn verify_props_against(
	el: &BsxElement,
	tag: &str,
	schema: &ValueSchema,
	cx: &mut TemplateContext,
) -> Result<()> {
	// resolve composable references against the schema registry snapshot.
	let mut resolved = cx
		.entity
		.world_scope(|world| world.get_resource::<SchemaRegistry>().cloned())
		.map(|registry| registry.resolve(schema))
		.unwrap_or_else(|| schema.clone());
	// a tag may carry attributes beyond its declared props (eg `class`), which are
	// forwarded rather than rejected, so prop validation permits extra keys.
	if let ValueSchema::Struct(struct_schema) = &mut resolved {
		struct_schema.allow_additional = true;
	}
	let mut props = props_value(el);
	// `validate` is async-shaped but resolves in one poll without an executor, so
	// `async_ext::block_on` drives it on both std and no_std.
	let errors = async_ext::block_on(resolved.validate(&mut props));
	if errors.is_empty() {
		Ok(())
	} else {
		let report = errors
			.iter()
			.map(|error| error.to_string())
			.collect::<Vec<_>>()
			.join(", ");
		bevybail!("template `{tag}` prop validation failed: {report}")
	}
}

/// Build a [`Value::Map`] of a tag's literal prop attributes, for schema
/// validation. Directives, spreads, references and entity refs are skipped, as
/// they are not plain prop values.
fn props_value(el: &BsxElement) -> Value {
	let mut map = Map::default();
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		if let Some(value) = attr_prop_value(&attr.value) {
			map.insert(SmolStr::from(attr.key.as_str()), value);
		}
	}
	Value::Map(map)
}

/// The plain [`Value`] of a prop attribute, or `None` when it is not a literal
/// (a `#`reference or `$`entity ref carries no inline value to verify).
fn attr_prop_value(value: &AttrValue) -> Option<Value> {
	match value {
		AttrValue::Flag => Some(Value::Bool(true)),
		AttrValue::Str(string) => Some(Value::Str(string.into())),
		AttrValue::Expr(ValueExpr::Literal(literal)) => literal_prop_value(literal),
		// a binding or entity ref is not a plain prop value
		AttrValue::Expr(_) | AttrValue::Spread(_) => None,
	}
}

/// Convert a [`DataLiteral`] to a [`Value`] for validation, mapping an enum
/// variant to its serde-tagged form.
fn literal_prop_value(literal: &DataLiteral) -> Option<Value> {
	match literal {
		DataLiteral::Scalar(value) => Some(value.clone()),
		DataLiteral::List(items) => items
			.iter()
			.map(literal_prop_value)
			.collect::<Option<Vec<_>>>()
			.map(Value::List),
		DataLiteral::Struct(fields) => {
			let mut map = Map::default();
			for (key, item) in fields {
				map.insert(SmolStr::from(key.as_str()), literal_prop_value(item)?);
			}
			Some(Value::Map(map))
		}
		DataLiteral::Enum(named) => named_prop_value(named),
		// an entity reference is not a plain value
		DataLiteral::EntityRef(_) => None,
	}
}

/// Convert a named literal (enum variant or struct) to its serde-tagged [`Value`].
fn named_prop_value(named: &NamedLiteral) -> Option<Value> {
	match &named.fields {
		// a unit variant is its bare name
		NamedFields::Unit => Some(Value::Str(named.name.as_str().into())),
		// a tuple/struct variant is `{ "Variant": payload }`
		NamedFields::Tuple(items) => {
			let payload = items
				.iter()
				.map(literal_prop_value)
				.collect::<Option<Vec<_>>>()
				.map(Value::List)?;
			let mut map = Map::default();
			map.insert(SmolStr::from(named.name.as_str()), payload);
			Some(Value::Map(map))
		}
		NamedFields::Struct(fields) => {
			let mut payload = Map::default();
			for (key, item) in fields {
				payload
					.insert(SmolStr::from(key.as_str()), literal_prop_value(item)?);
			}
			let mut map = Map::default();
			map.insert(SmolStr::from(named.name.as_str()), Value::Map(payload));
			Some(Value::Map(map))
		}
	}
}

/// A template's `bx:schema` declaration: an inline JSON schema, a remote schema
/// referenced by `src` (resolved asynchronously), or none.
#[derive(Debug, Clone, Default)]
pub enum SchemaDirective {
	/// No `bx:schema` block.
	#[default]
	None,
	/// An inline JSON schema, parsed at registration.
	Inline(ValueSchema),
	/// A remote schema URL, fetched asynchronously and awaited by `LoadTemplate`.
	Remote(SmolStr),
}

/// Extract the `bx:schema` directive declared among `nodes`: the first
/// `<script bx:schema>` block, inline (a JSON body) or remote (a `src` url).
pub fn extract_schema_directive(nodes: &[BsxNode]) -> SchemaDirective {
	nodes
		.iter()
		.find_map(|node| {
			let BsxNode::Element(el) = node else {
				return None;
			};
			if !is_schema_block(el) {
				return None;
			}
			// a `src` makes it remote; otherwise the raw-text body is inline JSON.
			if let Some(src) = string_attr(el, "src") {
				return Some(SchemaDirective::Remote(SmolStr::from(src.as_str())));
			}
			let json = schema_block_body(el)?;
			parse_json_schema(&json).ok().map(SchemaDirective::Inline)
		})
		.unwrap_or(SchemaDirective::None)
}

/// Extract the inline prop schema declared by a `<script bx:schema>` block, if
/// present (the non-remote case).
pub fn extract_bx_schema(nodes: &[BsxNode]) -> Option<ValueSchema> {
	match extract_schema_directive(nodes) {
		SchemaDirective::Inline(schema) => Some(schema),
		_ => None,
	}
}

/// The string value of a literal-string attribute on `el`, if present.
fn string_attr(el: &BsxElement, key: &str) -> Option<String> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != key {
			return None;
		}
		match &attr.value {
			AttrValue::Str(string) => Some(string.clone()),
			_ => None,
		}
	})
}

/// Remove every `<script bx:schema>` block from `nodes`, so a template's body
/// does not render its schema declaration.
pub fn strip_schema_blocks(nodes: Vec<BsxNode>) -> Vec<BsxNode> {
	nodes
		.into_iter()
		.filter(|node| {
			!matches!(node, BsxNode::Element(el) if is_schema_block(el))
		})
		.collect()
}

/// Whether `el` is a `<script ... bx:schema>` block.
fn is_schema_block(el: &BsxElement) -> bool {
	el.tag == "script"
		&& el.attributes.iter().any(|attr| attr.key == "bx:schema")
}

/// The raw-text body of a `<script>` schema block.
fn schema_block_body(el: &BsxElement) -> Option<String> {
	el.children.iter().find_map(|child| match child {
		BsxNode::Text(text) => Some(text.clone()),
		_ => None,
	})
}

/// Parse a `bx:schema` JSON block into a [`ValueSchema::Struct`].
///
/// The authoring surface is a JSON object mapping each prop name to a type
/// descriptor, which is either:
/// - a string naming a primitive (`"string"`, `"i64"`, `"u64"`, `"f64"`,
///   `"bool"`) or another template/type, lowering to a composable
///   [`ValueSchema::Reference`];
/// - an object `{ "type": <descriptor>, "required": bool, "optional": bool,
///   "items": <descriptor> }`, for required/optional fields and lists.
///
/// A field is optional unless `"required": true`. An `"optional": true` or an
/// `Option`-style descriptor wraps the field in [`ValueSchema::Optional`].
#[cfg(feature = "json")]
pub fn parse_json_schema(json: &str) -> Result<ValueSchema> {
	let root: serde_json::Value = serde_json::from_str(json)?;
	let serde_json::Value::Object(map) = root else {
		bevybail!("a `bx:schema` block must be a JSON object of props");
	};
	let mut fields = Vec::new();
	for (key, descriptor) in map {
		let (schema, required) = parse_descriptor(&descriptor)?;
		fields.push(NamedFieldSchema {
			key: SmolStr::from(key.as_str()),
			required,
			label: None,
			description: None,
			schema,
		});
	}
	Ok(ValueSchema::Struct(StructSchema {
		name: None,
		allow_additional: false,
		fields,
	}))
}

/// Fallback for builds without the `json` feature: a `bx:schema` block cannot be
/// parsed, so it is treated as absent.
#[cfg(not(feature = "json"))]
pub fn parse_json_schema(_json: &str) -> Result<ValueSchema> {
	bevybail!("parsing a `bx:schema` block requires the `json` feature")
}

/// Parse one field descriptor, returning its schema and whether it is required.
#[cfg(feature = "json")]
fn parse_descriptor(descriptor: &serde_json::Value) -> Result<(ValueSchema, bool)> {
	match descriptor {
		// a bare string names a primitive or references another template/type
		serde_json::Value::String(name) => Ok((descriptor_type(name), false)),
		serde_json::Value::Object(map) => {
			let required = map
				.get("required")
				.and_then(serde_json::Value::as_bool)
				.unwrap_or(false);
			let optional = map
				.get("optional")
				.and_then(serde_json::Value::as_bool)
				.unwrap_or(false);
			let mut schema = match (map.get("items"), map.get("type")) {
				// a list of the item descriptor
				(Some(items), _) => {
					let (item, _) = parse_descriptor(items)?;
					ValueSchema::List(ListSchema {
						item: Box::new(item),
						min_items: None,
						max_items: None,
						unique: false,
					})
				}
				(None, Some(serde_json::Value::String(name))) => {
					descriptor_type(name)
				}
				(None, Some(nested)) => parse_descriptor(nested)?.0,
				(None, None) => ValueSchema::Any,
			};
			if optional {
				schema = ValueSchema::Optional(Box::new(schema));
			}
			Ok((schema, required))
		}
		other => bevybail!("unsupported `bx:schema` descriptor: {other}"),
	}
}

/// Map a descriptor name to a primitive schema, or a composable reference to
/// another template/type's schema.
#[cfg(feature = "json")]
fn descriptor_type(name: &str) -> ValueSchema {
	match name {
		"string" | "str" => ValueSchema::String(StringSchema::default()),
		"i64" | "int" | "i32" => ValueSchema::I64(I64Schema::default()),
		"u64" | "uint" | "u32" => ValueSchema::U64(U64Schema::default()),
		"f64" | "float" | "f32" => ValueSchema::F64(F64Schema::default()),
		"bool" => ValueSchema::Bool(BoolSchema::default()),
		"any" => ValueSchema::Any,
		"null" => ValueSchema::Null,
		// anything else references another template/type schema by name
		other => ValueSchema::Reference(SmolStr::from(other)),
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	fn element(attrs: &[(&str, AttrValue)]) -> BsxElement {
		BsxElement {
			tag: "Foo".into(),
			attributes: attrs
				.iter()
				.map(|(key, value)| BsxAttribute {
					key: key.to_string(),
					value: value.clone(),
				})
				.collect(),
			children: Vec::new(),
			self_closing: true,
		}
	}

	#[beet_core::test]
	fn props_value_collects_literals() {
		let el = element(&[
			("label", AttrValue::Str("hi".into())),
			("count", AttrValue::Expr(ValueExpr::Literal(DataLiteral::Scalar(Value::Int(3))))),
			("bx:scope", AttrValue::Str("x".into())),
		]);
		let Value::Map(map) = props_value(&el) else {
			panic!("expected map");
		};
		// the two literal props are collected, the directive is skipped
		map.0.len().xpect_eq(2);
		map.0.get("label").unwrap().clone().xpect_eq(Value::Str("hi".into()));
		map.0.get("count").unwrap().clone().xpect_eq(Value::Int(3));
	}
}

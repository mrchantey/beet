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
//! composable [`SchemaRef::Name`]s against the [`SchemaRegistry`]), and
//! validates, surfacing a missing required field or a type mismatch as a graceful
//! error that rides [`TemplateError`](beet_core::prelude::TemplateError) on the
//! root rather than panicking.

use crate::bsx::resolve::is_directive;
use crate::prelude::*;
use bevy::ecs::template::TemplateContext;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;
use bevy::reflect::TypeRegistry;
use core::any::TypeId;

/// Verify the props supplied to `tag` against its registered prop schema.
///
/// Builds a [`Value::Map`] from the element's literal prop attributes, resolves
/// the template's [`ValueSchema`] against the world's [`SchemaRegistry`] (so a
/// composable [`SchemaRef::Name`] is substituted), and validates. Returns
/// an `Err` describing the failures, which rides the root's `TemplateError`.
///
/// A tag with no registered schema, or props that are not plain values (an entity
/// reference, a field binding), is left unverified, since those resolve by other
/// means.
pub(in crate::bsx) fn verify_props(
	el: &BsxElement,
	tag: &str,
	app_registry: &AppTypeRegistry,
	cx: &mut TemplateContext,
) -> Result<()> {
	let Some(schema) = ValueSchema::template_by_name(app_registry, tag) else {
		return Ok(());
	};
	let mut props = props_value(el);
	// a Rust template names each prop's type, so an authored spelling (a
	// `"30s"` duration, a `"guestbook.*"` glob, a `"false"` bool) parses here,
	// before the structural validation below, which only knows the type's
	// STRUCTURE and would reject the very form the coercion layer was written
	// to accept.
	let mut settled =
		parse_literal_props(&app_registry.read(), tag, &mut props)?;
	settled.extend(props_resolved_elsewhere(el));
	validate_props(tag, &schema, props, &settled, cx)
}

/// Verify a tag's props against an explicit `schema`, the shared path for both a
/// Rust template (schema looked up via [`ValueSchema::template_by_name`]) and a BSX
/// template (schema from its `bx:schema` block).
///
/// Resolves composable [`SchemaRef::Name`]s against the world's
/// [`SchemaRegistry`], then validates. A missing required field or type mismatch
/// is an `Err` that rides the root's `TemplateError`.
pub(in crate::bsx) fn verify_props_against(
	el: &BsxElement,
	tag: &str,
	schema: &ValueSchema,
	cx: &mut TemplateContext,
) -> Result<()> {
	// a BSX-authored template is typed by its `bx:schema` block alone, so there
	// are no Rust prop types to parse an authored spelling into.
	validate_props(
		tag,
		schema,
		props_value(el),
		&props_resolved_elsewhere(el),
		cx,
	)
}

/// Validate `props` against `schema`, skipping the keys that are already
/// settled: one a [`LiteralParser`] accepted, and one that resolves by other
/// means (see [`props_resolved_elsewhere`]).
fn validate_props(
	tag: &str,
	schema: &ValueSchema,
	mut props: Value,
	settled: &HashSet<SmolStr>,
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
		// a prop its type's parser already accepted is settled: its authored
		// spelling is not its structure, so the structural field would reject
		// the very form the coercion layer was written for.
		struct_schema
			.fields
			.retain(|field| !settled.contains(field.key.as_str()));
	}
	if let Value::Map(map) = &mut props {
		map.0.retain(|key, _| !settled.contains(key.as_str()));
	}
	// `validate` is async-shaped but resolves in one poll without an executor, so
	// `try_block_on` drives it on both std and no_std.
	let errors = async_ext::try_block_on(resolved.validate(&mut props))?;
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

/// Parse each authored prop through its field type's [`LiteralParser`],
/// returning the keys that parsed.
///
/// The prop-verification end of the seam contract: a parser that accepts the
/// value settles the prop, one that errors is an authoring failure carrying the
/// parser's message, and one that declines (or a type with no entry) leaves the
/// prop to structural validation, so a `GlobFilter` struct literal still
/// validates the old way.
fn parse_literal_props(
	registry: &TypeRegistry,
	tag: &str,
	props: &mut Value,
) -> Result<HashSet<SmolStr>> {
	let mut parsed = HashSet::default();
	let (Value::Map(map), Some(TypeInfo::Struct(info))) = (
		props,
		ReflectTemplate::registration_named(registry, tag)
			.map(TypeRegistration::type_info),
	) else {
		return Ok(parsed);
	};
	for (key, value) in map.0.iter() {
		let Some(type_id) = info
			.field(key.as_str())
			.and_then(|field| field.type_info())
			.map(prop_target)
		else {
			continue;
		};
		match LiteralParser::parse_type(type_id, value) {
			Ok(Some(_)) => {
				parsed.insert(key.clone());
			}
			Ok(None) => {}
			Err(err) => bevybail!(
				"template `{tag}` prop validation failed: `{key}`: {err}"
			),
		}
	}
	Ok(parsed)
}

/// The [`TypeId`] a prop's authored value must parse into: the field's own
/// type with the `PropOpt`/`Option` wrappers a `#[template]` signature adds
/// peeled off, since neither changes how the value is spelled.
fn prop_target(mut info: &'static TypeInfo) -> TypeId {
	loop {
		let inner = match info {
			TypeInfo::TupleStruct(tuple)
				if tuple
					.type_path_table()
					.short_path()
					.starts_with("PropOpt<")
					&& tuple.field_len() == 1 =>
			{
				tuple.field_at(0).and_then(|field| field.type_info())
			}
			_ => reflect_ext::option_some_inner(info),
		};
		match inner {
			Some(inner) => info = inner,
			None => return info.type_id(),
		}
	}
}

/// Build a [`Value::Map`] of a tag's literal prop attributes, for schema
/// validation and as a props store's initial document (see
/// `resolve.rs::apply_props_store`). Directives, spreads, references and entity
/// refs are skipped, as they are not plain prop values.
pub(in crate::bsx) fn props_value(el: &BsxElement) -> Value {
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

/// The props a tag supplied that carry no inline value to verify: an entity ref
/// (`deploy={$up}`) or a field binding.
///
/// They ARE supplied, and the reflect patch resolves each through its own
/// machinery, so validation must treat them as settled rather than as absent.
/// Without this a `#[prop(required)]` field fed an entity ref reports as
/// missing, which is the one failure a required prop exists to prevent and the
/// one it cannot itself be at fault for.
pub(in crate::bsx) fn props_resolved_elsewhere(
	el: &BsxElement,
) -> HashSet<SmolStr> {
	el.attributes
		.iter()
		.filter(|attr| !is_directive(&attr.key) && !attr.key.is_empty())
		.filter(|attr| attr_prop_value(&attr.value).is_none())
		.map(|attr| SmolStr::from(attr.key.as_str()))
		.collect()
}

/// The plain [`Value`] of a prop attribute, or `None` when it is not a literal
/// (a `#`reference or `$`entity ref carries no inline value to verify).
fn attr_prop_value(value: &AttrValue) -> Option<Value> {
	match value {
		AttrValue::Flag => Some(Value::Bool(true)),
		AttrValue::Str(string) => Some(Value::Str(string.into())),
		AttrValue::Expr(ValueExpr::Literal(literal)) => {
			literal_prop_value(literal)
		}
		// a binding, entity ref, spread or style directive is not a plain prop
		// value
		AttrValue::Expr(_) | AttrValue::Spread(_) | AttrValue::Style { .. } => {
			None
		}
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
				map.insert(
					SmolStr::from(key.as_str()),
					literal_prop_value(item)?,
				);
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
	// reflection keys on the bare variant/type name, so a qualified path
	// (`ButtonVariant::Outlined`) reduces to its last segment (`Outlined`), the
	// markup twin of Rust accepting either form.
	let name = variant_name(&named.name);
	match &named.fields {
		// a unit variant is its bare name
		NamedFields::Unit => Some(Value::Str(name.into())),
		// a tuple/struct variant is `{ "Variant": payload }`
		NamedFields::Tuple(items) => {
			let payload = items
				.iter()
				.map(literal_prop_value)
				.collect::<Option<Vec<_>>>()
				.map(Value::List)?;
			let mut map = Map::default();
			map.insert(SmolStr::from(name), payload);
			Some(Value::Map(map))
		}
		NamedFields::Struct(fields) => {
			let mut payload = Map::default();
			for (key, item) in fields {
				payload.insert(
					SmolStr::from(key.as_str()),
					literal_prop_value(item)?,
				);
			}
			let mut map = Map::default();
			map.insert(SmolStr::from(name), Value::Map(payload));
			Some(Value::Map(map))
		}
	}
}

/// The bare variant/type name of a (possibly qualified) path: the segment after
/// the last `::`, eg `ButtonVariant::Outlined` -> `Outlined`.
fn variant_name(name: &str) -> &str { name.rsplit("::").next().unwrap_or(name) }

/// A template's `bx:schema` declaration: an inline JSON schema, a remote schema
/// referenced by `src` (resolved asynchronously), or none.
#[derive(Debug, Clone, Default)]
pub(in crate::bsx) enum SchemaDirective {
	/// No `bx:schema` block.
	#[default]
	None,
	/// An inline JSON schema, parsed at registration.
	Inline(ValueSchema),
	/// A remote schema URL, fetched asynchronously and awaited by `Ready`.
	Remote(SmolStr),
}

/// Extract the `bx:schema` directive declared among `nodes`: the first
/// `<script bx:schema>` block, inline (a JSON body) or remote (a `src` url).
///
/// A template declaring no block declares no schema; a block that IS declared
/// and does not parse is an error, never a silently unschema'd template whose
/// props then validate against nothing.
pub(in crate::bsx) fn extract_schema_directive(
	nodes: &[BsxNode],
) -> Result<SchemaDirective> {
	let Some(el) = nodes
		.iter()
		.filter_map(|node| match node {
			BsxNode::Element(el) => Some(el),
			_ => None,
		})
		.find(|el| is_schema_block(el))
	else {
		return SchemaDirective::None.xok();
	};
	// a `src` makes it remote; otherwise the raw-text body is inline JSON.
	if let Some(src) = string_attr(el, "src") {
		return SchemaDirective::Remote(SmolStr::from(src.as_str())).xok();
	}
	schema_block_body(el)
		.ok_or_else(|| bevyhow!("`bx:schema` block declares no schema body"))?
		.xmap(|json| ValueSchema::from_json_schema(&json))?
		.xmap(SchemaDirective::Inline)
		.xok()
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
pub(in crate::bsx) fn strip_schema_blocks(nodes: Vec<BsxNode>) -> Vec<BsxNode> {
	nodes
		.into_iter()
		.filter(
			|node| !matches!(node, BsxNode::Element(el) if is_schema_block(el)),
		)
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

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	fn element(attrs: &[(&str, AttrValue)]) -> BsxElement {
		BsxElement {
			tag: "Foo".into(),
			tag_literal: None,
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

	#[crate::test]
	fn props_value_collects_literals() {
		let el = element(&[
			("label", AttrValue::Str("hi".into())),
			(
				"count",
				AttrValue::Expr(ValueExpr::Literal(DataLiteral::Scalar(
					Value::Int(3),
				))),
			),
			("bx:scope", AttrValue::Str("x".into())),
		]);
		let Value::Map(map) = props_value(&el) else {
			panic!("expected map");
		};
		// the two literal props are collected, the directive is skipped
		map.0.len().xpect_eq(2);
		map.0
			.get("label")
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("hi".into()));
		map.0.get("count").unwrap().clone().xpect_eq(Value::Int(3));
	}

	/// A template's prop types, registered by short path exactly as
	/// `registration_named` resolves them.
	#[derive(Reflect, Default)]
	struct Widget {
		home: bool,
		expanded: PropOpt<bool>,
		label: String,
		read: GlobFilter,
		duration: Option<core::time::Duration>,
	}

	fn prop_registry() -> TypeRegistry {
		let mut registry = TypeRegistry::default();
		registry.register::<Widget>();
		registry
	}

	/// Parse the props of a `<Widget ..>` element through the pre-pass.
	fn parse_widget_props(
		attrs: &[(&str, AttrValue)],
	) -> Result<HashSet<SmolStr>> {
		parse_literal_props(
			&prop_registry(),
			"Widget",
			&mut props_value(&element(attrs)),
		)
	}

	/// A markup attribute is always a string, so a prop must verify in the form
	/// the apply layer coerces: a `"true"`/`"false"` bool
	/// (`<RouteSidebar home="false"/>`), a `"30s"` duration, and a bare glob
	/// pattern, each settled by its type's parser rather than rejected by the
	/// structural field it does not look like.
	#[crate::test]
	fn authored_spellings_verify() {
		parse_widget_props(&[
			("home", AttrValue::Str("false".into())),
			("expanded", AttrValue::Str("true".into())),
			("label", AttrValue::Str("true".into())),
			("read", AttrValue::Str("guestbook.*".into())),
			("duration", AttrValue::Str("30s".into())),
		])
		.unwrap()
		.len()
		.xpect_eq(5);
	}

	/// A prop shape the parser DECLINES is left to structural validation, so a
	/// `GlobFilter` struct literal still validates the old way.
	#[crate::test]
	fn a_declined_prop_stays_structural() {
		parse_widget_props(&[(
			"read",
			AttrValue::Expr(ValueExpr::Literal(DataLiteral::Struct(vec![(
				"exclude".into(),
				DataLiteral::List(vec![DataLiteral::Scalar(Value::Str(
					"blog/**".into(),
				))]),
			)]))),
		)])
		.unwrap()
		.is_empty()
		.xpect_true();
	}

	/// A malformed authored value is a verification failure carrying the
	/// parser's own message, not a fallthrough to a structural check that would
	/// pass it (the schema of a `Duration` prop is a plain string).
	#[crate::test]
	fn a_malformed_prop_names_itself() {
		parse_widget_props(&[("duration", AttrValue::Str("30".into()))])
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid duration");
		parse_widget_props(&[("home", AttrValue::Str("yes".into()))])
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid bool");
	}

	#[crate::test]
	fn qualified_unit_enum_prop_reduces_to_variant() {
		// `variant=ButtonVariant::Outlined` reflects by the bare variant name, so the
		// qualifying path is dropped (else the enum silently falls back to default).
		let el = element(&[(
			"variant",
			AttrValue::Expr(ValueExpr::Literal(DataLiteral::Enum(
				NamedLiteral {
					name: "ButtonVariant::Outlined".into(),
					fields: NamedFields::Unit,
				},
			))),
		)]);
		let Value::Map(map) = props_value(&el) else {
			panic!("expected map");
		};
		map.0
			.get("variant")
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Outlined".into()));
	}
	/// A prop fed an entity reference is SUPPLIED, not missing: the value
	/// carries no inline form to validate, and the reflect patch resolves it
	/// through the entity model instead. Without this a `#[prop(required)]`
	/// field fed a `$ref` reports as absent.
	#[crate::test]
	fn an_entity_ref_prop_is_settled_rather_than_missing() {
		let el = element(&[
			("deploy", AttrValue::Expr(ValueExpr::EntityRef("up".into()))),
			("label", AttrValue::Str("x".into())),
		]);
		super::props_resolved_elsewhere(&el)
			.into_iter()
			.map(|key| key.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["deploy".to_string()]);
	}
}

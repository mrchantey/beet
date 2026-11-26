//! Verifying the props a template tag passes against its registered schema.
//!
//! A schema types a template's props: the `#[template]`'s typed signature,
//! registered alongside its build bridge ([`ReflectTemplateSchema`]). The
//! check runs as a document lowers, so a missing required prop or a type
//! mismatch is an error of the document, naming the tag, before anything
//! spawns.

use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;
use core::any::TypeId;

/// Verify the props supplied to a template tag against its registered schema.
///
/// Builds a [`Value::Map`] from the element's literal prop attributes and
/// validates it. A template with no schema, or props that are not plain values
/// (an entity reference), is left unverified, since those resolve by other
/// means.
pub(super) fn verify_props(
	el: &BsxElement,
	registration: &TypeRegistration,
) -> Result<()> {
	let Some(schema) = registration.data::<ReflectTemplateSchema>() else {
		return Ok(());
	};
	let mut props = props_value(el);
	// a Rust template names each prop's type, so an authored spelling (a
	// `"30s"` duration, a `"guestbook.*"` glob, a `"false"` bool) parses here,
	// before the structural validation below, which only knows the type's
	// STRUCTURE and would reject the very form the coercion layer was written
	// to accept.
	let mut settled = parse_literal_props(registration, &el.tag, &mut props)?;
	settled.extend(props_resolved_elsewhere(el));
	validate_props(&el.tag, schema.schema(), props, &settled)
}

/// Validate `props` against `schema`, skipping the keys that are already
/// settled: one a [`LiteralParser`] accepted, and one that resolves by other
/// means (see [`props_resolved_elsewhere`]).
fn validate_props(
	tag: &str,
	schema: &ValueSchema,
	mut props: Value,
	settled: &HashSet<SmolStr>,
) -> Result<()> {
	let mut schema = schema.clone();
	// a tag may carry attributes beyond its declared props (eg `class`), which are
	// forwarded rather than rejected, so prop validation permits extra keys.
	if let ValueSchema::Struct(struct_schema) = &mut schema {
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
	let errors = async_ext::try_block_on(schema.validate(&mut props))?;
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
	registration: &TypeRegistration,
	tag: &str,
	props: &mut Value,
) -> Result<HashSet<SmolStr>> {
	let mut parsed = HashSet::default();
	let (Value::Map(map), TypeInfo::Struct(info)) =
		(props, registration.type_info())
	else {
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
/// type with any `Option` peeled off, since an optional prop is spelled exactly
/// as its inner value.
fn prop_target(mut info: &'static TypeInfo) -> TypeId {
	while let Some(inner) = reflect_ext::option_some_inner(info) {
		info = inner;
	}
	info.type_id()
}

/// Build a [`Value::Map`] of a tag's literal prop attributes. References are
/// skipped, as they are not plain prop values.
fn props_value(el: &BsxElement) -> Value {
	el.plain_attributes()
		.filter_map(|attr| {
			attr_prop_value(&attr.value)
				.map(|value| (SmolStr::from(attr.key.as_str()), value))
		})
		.collect::<Map>()
		.xmap(Value::Map)
}

/// The props a tag supplied that carry no inline value to verify, ie an entity
/// ref (`deploy={$up}`).
///
/// They ARE supplied, and the reflect patch resolves each through its own
/// machinery, so validation must treat them as settled rather than as absent.
/// Without this a `#[prop(required)]` field fed an entity ref reports as
/// missing, which is the one failure a required prop exists to prevent and the
/// one it cannot itself be at fault for.
fn props_resolved_elsewhere(el: &BsxElement) -> HashSet<SmolStr> {
	el.plain_attributes()
		.filter(|attr| attr_prop_value(&attr.value).is_none())
		.map(|attr| SmolStr::from(attr.key.as_str()))
		.collect()
}

/// The plain [`Value`] of a prop attribute, or `None` when it is not a literal.
fn attr_prop_value(value: &AttrValue) -> Option<Value> {
	match value {
		AttrValue::Flag => Some(Value::Bool(true)),
		AttrValue::Str(string) => Some(Value::Str(string.into())),
		AttrValue::Expr(ValueExpr::Literal(literal)) => {
			literal_prop_value(literal)
		}
		// a binding, entity ref or spread is not a plain prop value
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
				map.insert(
					SmolStr::from(key.as_str()),
					literal_prop_value(item)?,
				);
			}
			Some(Value::Map(map))
		}
		DataLiteral::Enum(named) => named_prop_value(named),
		// an entity reference is not a plain value
		DataLiteral::EntityRef(_) | DataLiteral::Binding(_) => None,
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

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	fn element(attrs: &[(&str, AttrValue)]) -> BsxElement {
		BsxElement::new(
			"Foo",
			attrs
				.iter()
				.map(|(key, value)| BsxAttribute::new(*key, value.clone()))
				.collect(),
			Vec::new(),
		)
	}

	#[beet_core::test]
	fn props_value_collects_literals() {
		let el = element(&[
			("label", AttrValue::Str("hi".into())),
			(
				"count",
				AttrValue::Expr(ValueExpr::Literal(DataLiteral::Scalar(
					Value::Int(3),
				))),
			),
			("slot", AttrValue::Str("x".into())),
		]);
		let Value::Map(map) = props_value(&el) else {
			panic!("expected map");
		};
		// the two literal props are collected, the slot routing is skipped
		map.0.len().xpect_eq(2);
		map.0
			.get("label")
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("hi".into()));
		map.0.get("count").unwrap().clone().xpect_eq(Value::Int(3));
	}

	/// A template's prop types.
	#[derive(Reflect, Default)]
	struct Widget {
		home: bool,
		expanded: Option<bool>,
		label: String,
		read: GlobFilter,
		duration: Option<core::time::Duration>,
	}

	/// Parse the props of a `<Widget ..>` element through the pre-pass.
	fn parse_widget_props(
		attrs: &[(&str, AttrValue)],
	) -> Result<HashSet<SmolStr>> {
		parse_literal_props(
			&TypeRegistration::of::<Widget>(),
			"Widget",
			&mut props_value(&element(attrs)),
		)
	}

	/// A markup attribute is always a string, so a prop must verify in the form
	/// the apply layer coerces: a `"true"`/`"false"` bool
	/// (`<RouteSidebar home="false"/>`), a `"30s"` duration, and a bare glob
	/// pattern, each settled by its type's parser rather than rejected by the
	/// structural field it does not look like.
	#[beet_core::test]
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

	/// The two named lists ARE a `GlobFilter`'s own spelling, so a denylist
	/// verifies through its parser rather than structurally.
	#[beet_core::test]
	fn a_struct_literal_spelling_parses() {
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
		.len()
		.xpect_eq(1);
	}

	/// A prop shape the parser DECLINES is left to structural validation, so a
	/// `GlobFilter` authored as its ordered rules still validates the old way.
	#[beet_core::test]
	fn a_declined_prop_stays_structural() {
		parse_widget_props(&[(
			"read",
			AttrValue::Expr(ValueExpr::Literal(DataLiteral::Struct(vec![(
				"rules".into(),
				DataLiteral::List(Vec::new()),
			)]))),
		)])
		.unwrap()
		.is_empty()
		.xpect_true();
	}

	/// A malformed authored value is a verification failure carrying the
	/// parser's own message, not a fallthrough to a structural check that would
	/// pass it (the schema of a `Duration` prop is a plain string).
	#[beet_core::test]
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

	#[beet_core::test]
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
	#[beet_core::test]
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

//! The arms of a [`DynamicForm`](super::DynamicForm) whose controls the bound
//! *value* decides rather than the schema: a payload-carrying enum, whose
//! payload is a different shape per variant, and a struct field whose schema a
//! sibling names ([`SchemaRef::AtField`]).
//!
//! Both ride a [`ValueRebuild`](super::value_rebuild::ValueRebuild) over their
//! own value, so the controls arrive with the value rather than with the choice
//! that asked for them. The unit enum sits here as the arm the same select
//! serves without a payload to rebuild, and the select itself is
//! [`variant_select`](super::variant_select)'s.
use super::composite_field::struct_row;
use super::composite_field::struct_title;
use super::field_layout::child_field;
use super::field_layout::group;
use super::field_layout::labeled;
use super::form::schema_field;
use super::value_rebuild::RebuildKey;
use super::value_rebuild::ValueRebuild;
use super::variant_select::variant_name;
use super::variant_select::variant_options;
use super::variant_select::variant_select;
use crate::prelude::*;
use beet_core::prelude::*;

/// The unit-enum arm: a [`Select`] bound to the field, with one `<option>` per
/// variant, its value the variant name (the serde form of a unit variant).
pub(super) fn unit_enum_field(
	schema: &EnumSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let name = field.field_path.to_string();
	labeled(label, rsx! {
		<Select field={field} name={name}>{variant_options(schema)}</Select>
	})
}

/// The payload-carrying enum arm: the variant [`Select`], and the selected
/// variant's own controls beneath it.
///
/// The payload is a different shape per variant, so the pair rides a
/// [`ValueRebuild`](super::value_rebuild::ValueRebuild) keyed on the variant the
/// bound value carries: choosing a variant writes that variant's zero into the
/// field, and the controls for it arrive with the value, not with the choice.
pub(super) fn enum_field(
	schema: &EnumSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let (owned, bound_field) = (schema.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		|value| vec![RebuildKey::Name(variant_name(value).unwrap_or_default())],
		move |resolver, value, _key| {
			variant_control(resolver, &owned, &bound_field, value, depth)
		},
	);
	// the holder carries the enum's own value, so it must be an element
	group(label, rsx! { <div {(field, rebuild)}/> })
}

/// One generation of a payload-carrying enum: the select showing the variant the
/// value carries, and the controls of that variant's payload.
fn variant_control(
	resolver: SchemaResolver,
	schema: &EnumSchema,
	field: &FieldRef,
	value: &Value,
	depth: usize,
) -> Snippet {
	let current = variant_name(value);
	let mut rows =
		vec![variant_select(resolver, schema, field, current.clone())];
	let payload = current.as_ref().and_then(|name| {
		schema
			.variants
			.iter()
			.find(|variant| &variant.name == name)
			.and_then(|variant| variant.payload.as_ref())
			.map(|payload| (name.clone(), payload))
	});
	if let Some((name, payload)) = payload {
		// the payload sits under the variant name, the externally tagged form
		let inner = child_field(field, name);
		rows.push(schema_field(resolver, payload, inner, None, depth + 1));
	}
	labeled(None, rows)
}

/// The struct arm for a struct whose fields name each other: the rows are a
/// [`ValueRebuild`](super::value_rebuild::ValueRebuild) over the struct's own
/// value, since a field saying "my schema is the one described at `schema`"
/// cannot be dispatched until that sibling's value is in hand.
///
/// The container binds and then descends, which is the rule
/// [`ValueSchema::bind`] states for validation, here at the widget layer: the
/// enclosing struct is the only scope that can answer an
/// [`AtField`](SchemaRef::AtField), so it is the only place the substitution can
/// happen. Each row is keyed by its field and, for a dependent one, the schema
/// it bound to, so a sibling's edit rebuilds the row it retyped and no other.
pub(super) fn bound_struct_field(
	schema: &StructSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let title = (depth > 0).then(|| struct_title(schema, &field, label));
	let (keyed, owned, bound_field) =
		(schema.clone(), schema.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		move |value| bound_rows(&keyed, value).map(|(key, _)| key).collect(),
		move |resolver, value, key| {
			bound_rows(&owned, value)
				.find(|(row, _)| row == key)
				.map(|(_, named)| {
					struct_row(resolver, &named, &bound_field, depth)
				})
				.unwrap_or_else(|| Snippet::from_bundle(()))
		},
	);
	// the holder carries the struct's own value, so it must be an element
	group(title, rsx! { <div {(field, rebuild)}/> })
}

/// Each field of `schema` with every [`SchemaRef::AtField`] substituted for
/// the schema the struct's own value describes (the widget twin of
/// validation's bind-then-descend), under the key its row reconciles by: a
/// plain field by its name, a dependent one by its name and the schema it
/// bound to, so the row is rebuilt when its type changes and reused when only
/// its value does.
fn bound_rows<'a>(
	schema: &'a StructSchema,
	value: &'a Value,
) -> impl Iterator<Item = (RebuildKey, NamedFieldSchema)> + 'a {
	let scope = value.as_map().ok();
	schema.fields.iter().map(move |named| {
		let bound = scope
			.map(|scope| named.schema.bind(scope))
			.unwrap_or_else(|| named.schema.clone());
		let key = match named.schema.binds_a_field() {
			true => RebuildKey::Bound {
				field: named.key.clone(),
				schema: format!("{bound:?}").into(),
			},
			false => RebuildKey::Name(named.key.clone()),
		};
		(key, NamedFieldSchema {
			schema: bound,
			..named.clone()
		})
	})
}

#[cfg(test)]
mod test {
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	enum Role {
		Engineer,
		Designer,
	}

	#[beet_core::test]
	fn a_unit_enum_dispatches_to_a_select() {
		test_ext::form_html(ValueSchema::of::<Role>())
			.xpect_contains("<select")
			.xpect_contains("<option value=\"Engineer\"")
			.xpect_contains("<option value=\"Designer\"");
	}

	/// A payload-carrying enum is a variant select plus the payload's own
	/// controls, and choosing another variant rewrites the field with that
	/// variant's zero and regenerates the controls under it.
	#[beet_core::test]
	fn a_payload_enum_selects_and_rebuilds() {
		let schema = ValueSchema::Enum(EnumSchema {
			name: Some("Status".into()),
			description: None,
			variants: vec![
				VariantSchema {
					name: "Active".into(),
					payload: None,
				},
				VariantSchema {
					name: "Snoozed".into(),
					payload: Some(ValueSchema::Struct(StructSchema {
						name: None,
						description: None,
						allow_additional: false,
						fields: vec![NamedFieldSchema::new(
							"days",
							ValueSchema::U64(default()),
						)],
					})),
				},
			],
		});
		let (mut world, root) = test_ext::build_form(
			schema,
			"field",
			value!({ "field": { "Snoozed": { "days": 2 } } }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("<option value=\"Active\"")
			.xpect_contains("name=\"field.Snoozed.days\"");

		// choosing a variant writes that variant's zero, and the payload
		// controls follow the value rather than the choice
		let select = test_ext::element_in(&mut world, "select");
		*world.entity_mut(select).get_mut::<Value>().unwrap() =
			Value::str("Active");
		test_ext::settle_world(&mut world);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "field": "Active" }));
		test_ext::render_world(&mut world, root)
			.xnot()
			.xpect_contains("field.Snoozed.days");
	}

	/// The `{ schema, value }` pair, the smallest dependent shape: `value`'s
	/// control is whatever `schema` currently describes.
	fn pair() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("Pair".into()),
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("schema", ValueSchema::meta()),
				NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
			],
		})
	}

	/// A field whose schema a sibling names gets the control that sibling's
	/// *value* asks for, which is the dependent arm made editable: the pair
	/// `{ schema, value }` renders a checkbox when the schema says `Bool`.
	#[beet_core::test]
	fn a_dependent_field_dispatches_on_its_sibling() {
		let (mut world, root) = test_ext::build_form(
			pair(),
			"field",
			value!({ "field": { "schema": { "Bool": {} }, "value": true } }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.value\"")
			.xpect_contains("type=\"checkbox\"");
	}

	/// ...and it *re*binds: writing another schema into the sibling regenerates
	/// the dependent control, which is the whole reason the arm rides a rebuild
	/// rather than being dispatched once at build time.
	#[beet_core::test]
	fn a_dependent_field_rebinds_when_its_sibling_changes() {
		let (mut world, root) = test_ext::build_form(
			pair(),
			"field",
			value!({ "field": { "schema": { "Bool": {} }, "value": true } }),
		);
		*world
			.entity_mut(root)
			.get_mut::<Document>()
			.unwrap()
			.0
			.get_mut("field")
			.unwrap()
			.get_mut("schema")
			.unwrap() = Value::from_serde(&ValueSchema::String(default())).unwrap();
		test_ext::settle_world(&mut world);

		// the dependent leaf alone retypes; the sibling's own controls (a
		// `StringSchema`'s flags) are its business
		let html = test_ext::render_world(&mut world, root);
		html.clone()
			.xpect_contains("type=\"text\" name=\"field.value\"");
		html.xnot()
			.xpect_contains("type=\"checkbox\" name=\"field.value\"");
	}

	/// ...and only the retyped row is rebuilt: the sibling that named the
	/// schema keeps its own row, entity and all, since its key never changed
	/// (the variant select *inside* it is that enum's own rebuild's business).
	#[beet_core::test]
	fn a_rebind_rebuilds_only_the_dependent_row() {
		let (mut world, root) = test_ext::build_form(
			pair(),
			"field",
			value!({ "field": { "schema": { "Bool": {} }, "value": true } }),
		);
		let sibling = test_ext::bound(&mut world, "field.schema");
		let dependent = test_ext::bound(&mut world, "field.value");
		*world
			.entity_mut(root)
			.get_mut::<Document>()
			.unwrap()
			.0
			.get_mut("field")
			.unwrap()
			.get_mut("schema")
			.unwrap() = Value::from_serde(&ValueSchema::String(default())).unwrap();
		test_ext::settle_world(&mut world);
		test_ext::bound(&mut world, "field.schema").xpect_eq(sibling);
		test_ext::bound(&mut world, "field.value").xpect_not_eq(dependent);
	}
}

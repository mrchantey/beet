//! [`DynamicForm`]: a `<form>` generated from a [`ValueSchema`], one control
//! per editable leaf.
//!
//! The bridge between the schema layer and the form controls. A schema kind
//! picks the widget, its hints and constraints supply the widget's bounds, and
//! a composite schema recurses into a group whose [`FieldRef`]s extend the
//! parent's path.
//!
//! It owns no state and performs no writes: every leaf binds one
//! `(document, field path)` through its own [`FieldRef`] and writes only its
//! local [`Value`], so the bidirectional syncs carry the edit and nothing here
//! ever holds a copy of a document. Each control also carries the leaf's path
//! as its `name`, so the whole form gathers as a typed [`Value`] map on
//! [`Submit`] — the commit boundary a transactional edit rides.
//!
//! Where a schema alone does not decide the controls, the *value* does, through
//! a [`ValueRebuild`]: a list's rows, a map's entries, an enum's payload and a
//! field whose schema a sibling names are all generated from the bound value and
//! regenerated when its shape changes. The schema-decided layout around them is
//! one generation of a [`SchemaRebuild`], so a committed schema edit regenerates
//! the form while the slot's authored children stay put.
//!
//! Surface-agnostic: the widgets it spawns are ordinary elements, so the same
//! form paints in a terminal and serves as HTML.
use super::collection_edit::CollectionEdit;
use super::collection_edit::add_entry_row;
use super::collection_edit::edit_button;
use super::variant_select::variant_name;
use super::variant_select::variant_select;
use crate::prelude::*;
use beet_core::prelude::*;

/// Cap on nested composite recursion. A schema graph is finite by construction
/// ([`SchemaResolver`] bounds a reference chain and leaves a cycle unresolved),
/// so this is a defensive bound; a deeper subtree renders as an
/// [`UneditableField`].
const MAX_DEPTH: usize = 8;

/// A `<form>` whose controls are generated from `schema`, each bound to its own
/// leaf of the document `field` points into:
///
/// - `Bool` -> [`Checkbox`]
/// - `I64`/`U64`/`F64` -> [`NumberField`], carrying the schema's `Min`/`Max`/
///   `Step` constraints as bounds
/// - `String` -> [`TextField`], masked when `sensitive`, a [`TextArea`] when
///   `multiline`
/// - `Optional` -> its inner schema (a missing value reads as empty)
/// - `Reference` -> the schema it names, resolved against the [`SchemaRegistry`]
/// - `Struct` -> a labelled group, one nested control per field, each
///   [`FieldRef`] extending this one's path. At the top level the fields are the
///   form's own rows; nested, they sit in an open `<details>` disclosure.
/// - `Tuple` -> the same group over its elements, labelled by position
/// - `List` -> one generated control per item, a remove button beside each and
///   an add button after them, appending the item schema's
///   [`default_value`](ValueSchema::default_value)
/// - `Map` -> the same over its entries, each labelled by its key, added under a
///   key typed beside the add button
/// - `Enum` -> a [`Select`] of variant names; a payload-carrying variant renders
///   its payload's own controls beside it, regenerated when the variant changes
/// - anything else (`Entity`/`Bytes`/`Any`/an unresolved `Reference`) -> a
///   read-only [`UneditableField`], since no control can produce a valid value
///   for it. An `Entity` reference wants the picker item 18 names.
///
/// The default slot lands inside the `<form>` after the generated controls, ie
/// where a submit [`Button`] goes.
///
/// ```rsx
/// <DynamicForm schema={ValueSchema::of::<TodoItem>()} field={FieldRef::new("draft")}>
///   <Button>"Add"</Button>
/// </DynamicForm>
/// ```
#[template(system)]
pub fn DynamicForm(
	/// The schema of the value this form edits. Omitted, the form edits under
	/// the schema the bound document declares at `field`, which is the authored
	/// form for a document loaded out of a store.
	#[prop]
	schema: Option<ValueSchema>,
	/// The document field the form edits: the path every generated control's
	/// own [`FieldRef`] extends. Defaults to the whole document.
	#[prop]
	field: FieldRef,
	/// The by-name registry a [`SchemaRef::Name`] resolves against;
	/// absent until [`DocumentPlugin`] has initialized it, which defers every
	/// reference to an [`UneditableField`] exactly as an unregistered name does.
	schemas: Option<Res<SchemaRegistry>>,
) -> impl Bundle {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	// the controls are one generation, respawned when a committed schema edit
	// changes what this schema resolves to; the slot's children are its siblings
	let controls = {
		let field = field.clone();
		move |resolver: SchemaResolver, schema: &ValueSchema| {
			schema_field(resolver, schema, field.clone(), None, 0)
		}
	};
	let source = match schema {
		Some(schema) => SchemaSource::Authored(schema),
		None => SchemaSource::Document(field),
	};
	rsx! {
		<Form>
			{SchemaRebuild::new(resolver, source, controls).holder(resolver)}
			<Slot/>
		</Form>
	}
}

/// Marks a [`DynamicForm`] leaf whose schema has no editing widget, naming the
/// kind that found none (`"Entity"`, an unresolved `"Ref"`).
///
/// The leaf still renders, read-only, so a form keeps its shape with only the
/// editing missing — the same bargain an unregistered tag strikes. The mark sits
/// on the bound leaf itself, so one `(&FieldRef, &UneditableField)` query
/// answers both which leaves lost their control and where each one binds.
///
/// It carries the *kind*, not the schema. A `#[template]` expands away at build,
/// so nothing survives it holding the schema this form walked, and a copy here
/// would be an unowned second one; the authoritative per-leaf schema arrives
/// from the document side instead, seeded onto every bound field by
/// `sync_schema` from the document's own [`DocumentSchema`].
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct UneditableField(pub SmolStr);

/// `(min, max, step)` as `f64` from a numeric schema's constraints, the last of
/// each kind winning. A macro because the three numeric schemas share the shape
/// but not the constraint type.
macro_rules! number_bounds {
	($schema:expr, $constraint:ident) => {{
		let mut bounds: (Option<f64>, Option<f64>, Option<f64>) =
			(None, None, None);
		for constraint in &$schema.constraints {
			match constraint {
				$constraint::Min(min) => bounds.0 = Some(min.value as f64),
				$constraint::Max(max) => bounds.1 = Some(max.value as f64),
				$constraint::Step(step) => bounds.2 = Some(step.value as f64),
			}
		}
		bounds
	}};
}

/// One dispatched leaf. Returns a [`Snippet`] because each arm builds a
/// differently-shaped tree, which is also what lets the composite arms recurse.
///
/// `depth` counts *composite nesting* — a struct, tuple, list, map or enum
/// payload, the positions the walk can descend through — so a reference hop or
/// an `Optional` unwrap is the same leaf seen more precisely and neither
/// consumes budget, and depth `0` stays "the form's own top level".
fn schema_field<'a>(
	resolver: SchemaResolver<'a>,
	schema: &'a ValueSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	match schema {
		ValueSchema::Bool(_) => bool_field(field, label),
		ValueSchema::I64(schema) => {
			number_field(number_bounds!(schema, I64Constraint), field, label)
		}
		ValueSchema::U64(schema) => {
			number_field(number_bounds!(schema, U64Constraint), field, label)
		}
		ValueSchema::F64(schema) => {
			number_field(number_bounds!(schema, F64Constraint), field, label)
		}
		ValueSchema::String(schema) => string_field(schema, field, label),
		// null is one of the values, which a control's empty state already is
		ValueSchema::Optional(inner) => {
			schema_field(resolver, inner, field, label, depth)
		}
		// the registry's schema is borrowed, never copied out: a reference hop
		// dispatches on the schema in place
		ValueSchema::Ref(SchemaRef::Name(name)) => {
			match resolver.schema(name) {
				Some(resolved) => {
					schema_field(resolver, resolved, field, label, depth)
				}
				// still arriving, or never coming: loud, not silently empty
				None => uneditable(schema, field, label),
			}
		}
		// a composite deeper than the budget is the one place the walk gives up
		_ if depth >= MAX_DEPTH && schema.is_composite() => {
			uneditable(schema, field, label)
		}
		// a payload carries data no `<option>` value can express, so only an
		// all-unit enum is a plain select
		ValueSchema::Enum(schema)
			if schema
				.variants
				.iter()
				.all(|variant| variant.payload.is_none()) =>
		{
			unit_enum_field(schema, field, label)
		}
		ValueSchema::Enum(schema) => enum_field(schema, field, label, depth),
		// a field naming a sibling cannot be dispatched until that sibling's
		// value is in hand, so the container binds before it descends
		ValueSchema::Struct(schema)
			if schema
				.fields
				.iter()
				.any(|named| named.schema.binds_a_field()) =>
		{
			bound_struct_field(schema, field, label, depth)
		}
		ValueSchema::Struct(schema) => {
			struct_field(resolver, schema, field, label, depth)
		}
		ValueSchema::Tuple(schema) => {
			tuple_field(resolver, schema, field, label, depth)
		}
		ValueSchema::List(schema) => {
			list_field(resolver, schema, field, label, depth)
		}
		ValueSchema::Map(schema) => {
			map_field(resolver, schema, field, label, depth)
		}
		_ => uneditable(schema, field, label),
	}
}

/// The boolean arm: the [`Checkbox`], the one control that produces a `Bool`.
fn bool_field(field: FieldRef, label: Option<String>) -> Snippet {
	labeled(
		label,
		widget(Checkbox {
			name: PropOpt(Some(field.field_path.to_string())),
			field: PropOpt(Some(field)),
		}),
	)
}

/// The numeric arm: a [`NumberField`] carrying whichever constraint bounds the
/// schema declares.
fn number_field(
	(min, max, step): (Option<f64>, Option<f64>, Option<f64>),
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	labeled(
		label,
		widget(NumberField {
			name: PropOpt(Some(field.field_path.to_string())),
			field: PropOpt(Some(field)),
			min: PropOpt(min),
			max: PropOpt(max),
			step: PropOpt(step),
			..default()
		}),
	)
}

/// The string arm: a [`TextArea`] for multiline prose, else a [`TextField`],
/// masked when the schema marks the value sensitive.
fn string_field(
	schema: &StringSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let name = PropOpt(Some(field.field_path.to_string()));
	let widget = match schema.multiline {
		true => widget(TextArea {
			name,
			field: PropOpt(Some(field)),
			..default()
		}),
		false => widget(TextField {
			name,
			field: PropOpt(Some(field)),
			sensitive: schema.sensitive,
			..default()
		}),
	};
	labeled(label, widget)
}

/// The unit-enum arm: a [`Select`] bound to the field, with one `<option>` per
/// variant, its value the variant name (the serde form of a unit variant).
fn unit_enum_field(
	schema: &EnumSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let name = field.field_path.to_string();
	labeled(label, rsx! {
		<Select field={field} name={name}>{variant_options(schema)}</Select>
	})
}

/// One `<option>` per variant, shared by the unit-enum select and the
/// payload-carrying one.
pub(super) fn variant_options(schema: &EnumSchema) -> Vec<Snippet> {
	schema
		.variants
		.iter()
		.map(|variant| {
			let variant = variant.name.to_string();
			rsx! { <option value=variant.clone()>{variant}</option> }
				.any_snippet()
		})
		.collect()
}

/// The payload-carrying enum arm: the variant [`Select`], and the selected
/// variant's own controls beneath it.
///
/// The payload is a different shape per variant, so the pair rides a
/// [`ValueRebuild`] keyed on the variant the bound value carries: choosing a
/// variant writes that variant's zero into the field, and the controls for it
/// arrive with the value, not with the choice.
fn enum_field(
	schema: &EnumSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let (owned, bound_field) = (schema.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		|value| variant_name(value).unwrap_or_default(),
		move |resolver, value| {
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

/// The struct arm: one nested control per named field, each [`FieldRef`]
/// extending this one's path and labelled by the field's label hint (else its
/// key). The form's own top level *is* the group, so only a nested struct wraps
/// its rows in a disclosure.
fn struct_field<'a>(
	resolver: SchemaResolver<'a>,
	schema: &'a StructSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let rows = struct_rows(resolver, schema, &field, depth);
	// the form's own top level is already the group
	match depth {
		0 => labeled(None, rows),
		_ => group(Some(struct_title(schema, &field, label)), rows),
	}
}

/// The struct arm for a struct whose fields name each other: the rows are a
/// [`ValueRebuild`] over the struct's own value, since a field saying "my schema
/// is the one described at `schema`" cannot be dispatched until that sibling's
/// value is in hand.
///
/// The container binds and then descends, which is the rule
/// [`ValueSchema::bind`] states for validation, here at the widget layer: the
/// enclosing struct is the only scope that can answer an
/// [`AtField`](SchemaRef::AtField), so it is the only place the substitution can
/// happen.
fn bound_struct_field(
	schema: &StructSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let title = (depth > 0).then(|| struct_title(schema, &field, label));
	let (shape_schema, owned, bound_field) =
		(schema.clone(), schema.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		move |value| bound_shape(&shape_schema, value),
		move |resolver, value| {
			let bound = bind_fields(&owned, value);
			labeled(None, struct_rows(resolver, &bound, &bound_field, depth))
		},
	);
	// the holder carries the struct's own value, so it must be an element
	group(title, rsx! { <div {(field, rebuild)}/> })
}

/// One control per named field, each [`FieldRef`] extending the struct's path.
fn struct_rows<'a>(
	resolver: SchemaResolver<'a>,
	schema: &'a StructSchema,
	field: &FieldRef,
	depth: usize,
) -> Vec<Snippet> {
	schema
		.fields
		.iter()
		.map(|named| {
			let label = named.label.as_ref().unwrap_or(&named.key).to_string();
			schema_field(
				resolver,
				&named.schema,
				child_field(field, named.key.clone()),
				Some(label),
				depth + 1,
			)
		})
		.collect()
}

/// The title a nested struct's disclosure wears: its label hint, else the name
/// the schema declares for itself, else the path it binds.
fn struct_title(
	schema: &StructSchema,
	field: &FieldRef,
	label: Option<String>,
) -> String {
	label
		.or_else(|| schema.name.as_ref().map(|name| name.to_string()))
		.unwrap_or_else(|| field.field_path.to_string())
}

/// Every [`SchemaRef::AtField`] in `schema`'s fields substituted with the schema
/// the struct's own value describes, the widget twin of validation's
/// bind-then-descend.
fn bind_fields(schema: &StructSchema, value: &Value) -> StructSchema {
	let Ok(scope) = value.as_map() else {
		return schema.clone();
	};
	StructSchema {
		fields: schema
			.fields
			.iter()
			.map(|named| NamedFieldSchema {
				schema: named.schema.bind(scope),
				..named.clone()
			})
			.collect(),
		..schema.clone()
	}
}

/// The shape a bound struct's controls are decided by: what its own value says
/// its dependent fields are. Anything else the value holds is a leaf's business.
fn bound_shape(schema: &StructSchema, value: &Value) -> SmolStr {
	let bound = bind_fields(schema, value);
	schema
		.fields
		.iter()
		.zip(bound.fields.iter())
		.filter(|(named, _)| named.schema.binds_a_field())
		.map(|(named, bound)| format!("{}={:?}", named.key, bound.schema))
		.collect::<Vec<_>>()
		.join(";")
		.into()
}

/// The tuple arm: one control per element, labelled by the element's
/// description hint else its position. A tuple's arity is its schema's, so it
/// has no add or remove.
fn tuple_field<'a>(
	resolver: SchemaResolver<'a>,
	schema: &'a TupleSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let rows = schema
		.fields
		.iter()
		.enumerate()
		.map(|(index, unnamed)| {
			let label = unnamed
				.description
				.as_ref()
				.map(|description| description.to_string())
				.unwrap_or_else(|| index.to_string());
			schema_field(
				resolver,
				&unnamed.schema,
				child_field(&field, index),
				Some(label),
				depth + 1,
			)
		})
		.collect::<Vec<_>>();
	group(label, rows)
}

/// The list arm: one control per item with a remove button beside it, and an add
/// button appending the item schema's zero.
///
/// The rows ride a [`ValueRebuild`] keyed on the list's *length*, so adding or
/// removing an item regenerates them while editing one is its own control's
/// business.
fn list_field(
	resolver: SchemaResolver,
	schema: &ListSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let (item, rows_field) = (schema.item.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		|value| item_count(value).to_string().into(),
		move |resolver, value| {
			(0..item_count(value))
				.map(|index| {
					list_row(resolver, &item, &rows_field, index, depth)
				})
				.collect::<Vec<_>>()
				.xmap(|rows| labeled(None, rows))
		},
	);
	let zero = schema.item.default_value_in(resolver);
	group(label, rsx! {
		<div {(field.clone(), rebuild)}/>
		{edit_button("add", field, CollectionEdit::Push(zero))}
	})
}

/// One list row: the item's own controls, and the button that drops it.
fn list_row(
	resolver: SchemaResolver,
	item: &ValueSchema,
	field: &FieldRef,
	index: usize,
	depth: usize,
) -> Snippet {
	rsx! {
		<div>
			{schema_field(
				resolver,
				item,
				child_field(field, index),
				None,
				depth + 1,
			)}
			{edit_button(
				"remove",
				field.clone(),
				CollectionEdit::Remove(FieldSegment::index(index)),
			)}
		</div>
	}
	.any_snippet()
}

/// The map arm: one control per entry, labelled by its key, with a remove button
/// beside it and a key to type beside the add button.
///
/// The entries ride a [`ValueRebuild`] keyed on the map's *keys*, in sorted
/// order so a rebuild does not reshuffle a control's neighbours.
fn map_field(
	resolver: SchemaResolver,
	schema: &MapSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let (value_schema, entries_field) = (schema.value.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		|value| entry_keys(value).join(";").into(),
		move |resolver, value| {
			entry_keys(value)
				.into_iter()
				.map(|key| {
					map_entry(
						resolver,
						&value_schema,
						&entries_field,
						key,
						depth,
					)
				})
				.collect::<Vec<_>>()
				.xmap(|rows| labeled(None, rows))
		},
	);
	let zero = schema.value.default_value_in(resolver);
	group(label, rsx! {
		<div {(field.clone(), rebuild)}/>
		{add_entry_row(field, zero)}
	})
}

/// One map entry: the value's own controls under the key's label, and the button
/// that drops the entry.
fn map_entry(
	resolver: SchemaResolver,
	schema: &ValueSchema,
	field: &FieldRef,
	key: SmolStr,
	depth: usize,
) -> Snippet {
	rsx! {
		<div>
			{schema_field(
				resolver,
				schema,
				child_field(field, key.clone()),
				Some(key.to_string()),
				depth + 1,
			)}
			{edit_button(
				"remove",
				field.clone(),
				CollectionEdit::Remove(FieldSegment::ObjectKey(key)),
			)}
		</div>
	}
	.any_snippet()
}

/// The number of items a list-typed value holds, `0` for anything else (a field
/// the document has yet to answer).
fn item_count(value: &Value) -> usize {
	value.as_list().map(Vec::len).unwrap_or_default()
}

/// A map-typed value's keys, sorted, empty for anything else.
fn entry_keys(value: &Value) -> Vec<SmolStr> {
	value
		.as_map()
		.map(|map| {
			map.0
				.keys()
				.cloned()
				.collect::<Vec<_>>()
				.xtap(|keys| keys.sort())
		})
		.unwrap_or_default()
}

/// A child position of `field`: the same document, one segment deeper.
fn child_field(field: &FieldRef, segment: impl Into<FieldSegment>) -> FieldRef {
	FieldRef {
		document: field.document.clone(),
		field_path: field.field_path.with_pushed(segment),
		on_missing: default(),
	}
}

/// The read-only leaf for a schema with no control: a bound text node, so the
/// value still shows but nothing can type into it (a text node carries no
/// element, so the focus path cannot reach it). The gap is announced where it is
/// built and marked where it landed.
fn uneditable(
	schema: &ValueSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let kind = schema.variant_name();
	warn!(
		"DynamicForm: no control edits a `{kind}` field at `{}`, rendering it read-only",
		field.field_path
	);
	labeled(label, (field, UneditableField(SmolStr::new_static(kind))))
}

/// Wrap a control in a `<label>` row (the key above its value, per the form
/// rules), or pass it through unlabelled.
fn labeled<M>(label: Option<String>, widget: impl IntoSnippet<M>) -> Snippet {
	match label {
		Some(label) => rsx! { <label>{label}{widget}</label> }.any_snippet(),
		None => Snippet::from_bundle(widget.into_snippet()),
	}
}

/// Wrap a composite's rows in a titled disclosure, or pass them through when
/// there is no title (the form's own top level, which is already the group).
fn group<M>(title: Option<String>, rows: impl IntoSnippet<M>) -> Snippet {
	match title {
		Some(title) => rsx! {
			<details open>
				<summary>{title}</summary>
				{rows}
			</details>
		}
		.any_snippet(),
		None => Snippet::from_bundle(rows.into_snippet()),
	}
}

/// Lift a constructed widget into a child-position [`Snippet`], the
/// struct-literal twin of an `rsx!` `<Widget/>` tag — reached for here because a
/// dispatched arm passes `Option`s straight into optional props, which a tag
/// cannot express.
fn widget(template: impl BuildTemplate) -> Snippet {
	Snippet::from_bundle(template.into_snippet_bundle())
}

#[cfg(test)]
mod test {
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Render a `DynamicForm` for `schema` bound to a `"field"` key.
	fn render(schema: ValueSchema) -> String {
		test_ext::render_html(rsx! {
			<DynamicForm schema={schema} field={FieldRef::new("field")}/>
		})
	}

	/// Build a form over `schema` and `document`, settled.
	fn build(schema: ValueSchema, document: Value) -> (World, Entity) {
		let mut world = test_ext::form_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicForm schema={schema} field={FieldRef::new("field")}/>
				</div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(document));
		test_ext::settle_world(&mut world);
		(world, root)
	}

	#[beet_core::test]
	fn bool_dispatches_to_a_checkbox() {
		render(ValueSchema::Bool(default()))
			.xpect_contains("type=\"checkbox\"");
	}

	/// A number carries its constraint bounds onto the control, so the widget
	/// enforces what the schema declares without restating it.
	#[beet_core::test]
	fn numbers_dispatch_to_a_number_field_with_bounds() {
		render(ValueSchema::I64(I64Schema {
			constraints: vec![
				I64Constraint::Min(I64Min {
					value: 1,
					behavior: default(),
				}),
				I64Constraint::Max(I64Max {
					value: 9,
					behavior: default(),
				}),
				I64Constraint::Step(I64Step {
					value: 2,
					behavior: default(),
				}),
			],
		}))
		.xpect_contains("type=\"number\"")
		.xpect_contains("min=\"1\"")
		.xpect_contains("max=\"9\"")
		.xpect_contains("step=\"2\"");
	}

	#[beet_core::test]
	fn strings_dispatch_by_hint() {
		render(ValueSchema::String(default())).xpect_contains("type=\"text\"");
		render(ValueSchema::String(StringSchema::default().multiline()))
			.xpect_contains("<textarea");
		render(ValueSchema::String(StringSchema::default().sensitive()))
			.xpect_contains("type=\"password\"");
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	enum Role {
		Engineer,
		Designer,
	}

	#[beet_core::test]
	fn a_unit_enum_dispatches_to_a_select() {
		render(ValueSchema::of::<Role>())
			.xpect_contains("<select")
			.xpect_contains("<option value=\"Engineer\"")
			.xpect_contains("<option value=\"Designer\"");
	}

	/// A shape no control can produce a valid value for renders read-only and
	/// marked, rather than as a text box that would write a lie into the
	/// document. An unresolved reference lands here too, so a schema that never
	/// arrived is loud rather than an empty form.
	#[beet_core::test]
	fn unreachable_shapes_are_uneditable() {
		for schema in [
			ValueSchema::Entity(default()),
			ValueSchema::Any,
			ValueSchema::reference("NotRegistered"),
		] {
			let kind = schema.variant_name();
			let mut world = world_ext::ui_world();
			let root = world
				.spawn_template(rsx! {
					<DynamicForm schema={schema} field={FieldRef::new("field")}/>
				})
				.unwrap()
				.id();
			world.update_local();
			// the mark names the kind and sits on the leaf that binds
			world
				.query_once::<(&FieldRef, &UneditableField)>()
				.into_iter()
				.map(|(field, mark)| {
					(field.field_path.to_string(), mark.0.to_string())
				})
				.collect::<Vec<_>>()
				.xpect_eq(vec![("field".to_string(), kind.to_string())]);
			let html = test_ext::render_world(&mut world, root);
			html.clone().xnot().xpect_contains("<input");
			html.xnot().xpect_contains("<textarea");
		}
	}

	#[derive(Reflect)]
	struct Profile {
		name: String,
		count: i64,
		active: bool,
	}

	/// The top level is the form's own rows; a nested struct is a disclosure
	/// group, one labelled control per field dispatched by its own schema.
	#[beet_core::test]
	fn a_struct_recurses_with_labels() {
		let html = render(ValueSchema::Struct(StructSchema {
			name: Some("Outer".into()),
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"profile",
				ValueSchema::of::<Profile>(),
			)],
		}));
		html.clone()
			// the top-level field is a row, its nested struct a disclosure
			.xpect_contains("<details open")
			.xpect_contains("<summary>profile</summary>")
			.xpect_contains("name")
			.xpect_contains("count")
			.xpect_contains("type=\"text\"")
			.xpect_contains("type=\"number\"")
			.xpect_contains("type=\"checkbox\"");
		// the nested paths are the leaves' names, so a submit gathers them whole
		html.clone().xpect_contains("name=\"field.profile.name\"");
		// ...and the top level is the form itself, not a group inside it
		html.xnot().xpect_contains("<summary>Outer");
	}

	/// A field's label hint replaces its key as the visible label; the key is
	/// still what the path (and so the binding) uses.
	#[beet_core::test]
	fn a_label_hint_replaces_the_key() {
		render(ValueSchema::Struct(StructSchema {
			name: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("name", ValueSchema::String(default()))
					.with_label("Display Name"),
			],
		}))
		.xpect_contains("Display Name")
		.xpect_contains("name=\"field.name\"");
	}

	/// A reference resolves against the registry and dispatches to the schema it
	/// names, which is what lets a data document compose `Reference("TodoItem")`
	/// and still generate a real form.
	#[beet_core::test]
	fn a_reference_resolves_through_the_registry() {
		let mut world = world_ext::ui_world();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("Role", ValueSchema::of::<Role>());
		let root = world
			.spawn_template(rsx! {
				<DynamicForm
					schema={ValueSchema::reference("Role")}
					field={FieldRef::new("role")}
				/>
			})
			.unwrap()
			.id();
		test_ext::render_world(&mut world, root)
			.xpect_contains("<option value=\"Engineer\"");
	}

	/// One control per editable leaf, each binding its own path and nothing
	/// else: the binding contract `form_controls::conformance` fences, now
	/// generated from a schema rather than authored by hand.
	#[beet_core::test]
	fn each_leaf_binds_its_own_path() {
		let mut world = world_ext::ui_world();
		world
			.spawn_template(rsx! {
				<DynamicForm schema={ValueSchema::of::<Profile>()}/>
			})
			.unwrap();
		world.update_local();
		world
			.query_once::<(&Element, &FieldRef)>()
			.into_iter()
			.map(|(element, field)| {
				(field.field_path.to_string(), element.tag().to_string())
			})
			.collect::<Vec<_>>()
			.xtap(|bindings| bindings.sort())
			.xpect_eq(vec![
				("active".to_string(), "input".to_string()),
				("count".to_string(), "input".to_string()),
				("name".to_string(), "input".to_string()),
			]);
	}

	/// The generated form is an ordinary bound form: an edit reaches its own
	/// leaf of the document and no sibling, and the form holds no copy of it.
	#[beet_core::test]
	fn an_edit_reaches_its_own_leaf() {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div><DynamicForm schema={ValueSchema::of::<Profile>()}/></div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(value!({
			"name": "ada", "count": 1, "active": false
		})));
		world.update_local();

		// the checkbox is the only control that can produce a `Bool`
		let checkbox = world
			.query_once::<(Entity, &CheckboxInput)>()
			.into_iter()
			.map(|(entity, _)| entity)
			.next()
			.unwrap();
		*world.entity_mut(checkbox).get_mut::<Value>().unwrap() =
			Value::Bool(true);
		world.update_local();

		world
			.entity(root)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "name": "ada", "count": 1, "active": true }));
	}

	/// A list's items each get the control their item schema asks for, bound to
	/// their own index, so an edit reaches one row and no other.
	#[beet_core::test]
	fn a_list_generates_a_control_per_item() {
		let (mut world, root) = build(
			ValueSchema::of::<Vec<String>>(),
			value!({ "field": ["buy milk", "walk dog"] }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.[0]\"")
			.xpect_contains("name=\"field.[1]\"");

		let input = test_ext::elements_in(&mut world, "input")[1];
		*world.entity_mut(input).get_mut::<Value>().unwrap() =
			Value::str("walk cat");
		test_ext::settle_world(&mut world);
		world
			.entity(root)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "field": ["buy milk", "walk cat"] }));
	}

	/// A list of structs is a control per field per row, so the todo app's rows
	/// are editable rather than a read-only table.
	#[beet_core::test]
	fn a_list_of_structs_recurses_per_row() {
		let (mut world, root) = build(
			ValueSchema::of::<Vec<Profile>>(),
			value!({ "field": [{ "name": "ada", "count": 1, "active": true }] }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.[0].name\"")
			.xpect_contains("name=\"field.[0].count\"")
			.xpect_contains("name=\"field.[0].active\"");
	}

	/// A tuple is one control per element, labelled by position, and has no add
	/// or remove: its arity is its schema's.
	#[beet_core::test]
	fn a_tuple_generates_a_control_per_element() {
		let (mut world, root) = build(
			ValueSchema::of::<(String, i64)>(),
			value!({ "field": ["ada", 3] }),
		);
		let html = test_ext::render_world(&mut world, root);
		html.clone()
			.xpect_contains("name=\"field.[0]\"")
			.xpect_contains("name=\"field.[1]\"");
		html.xnot().xpect_contains("<button");
	}

	/// A map is one labelled control per entry, bound to its own key.
	#[beet_core::test]
	fn a_map_generates_a_control_per_entry() {
		let (mut world, root) = build(
			ValueSchema::Map(MapSchema {
				value: Box::new(ValueSchema::Bool(default())),
			}),
			value!({ "field": { "done": true, "urgent": false } }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.done\"")
			.xpect_contains("name=\"field.urgent\"");
	}

	/// A payload-carrying enum is a variant select plus the payload's own
	/// controls, and choosing another variant rewrites the field with that
	/// variant's zero and regenerates the controls under it.
	#[beet_core::test]
	fn a_payload_enum_selects_and_rebuilds() {
		let schema = ValueSchema::Enum(EnumSchema {
			name: Some("Status".into()),
			variants: vec![
				VariantSchema {
					name: "Active".into(),
					payload: None,
				},
				VariantSchema {
					name: "Snoozed".into(),
					payload: Some(ValueSchema::Struct(StructSchema {
						name: None,
						allow_additional: false,
						fields: vec![NamedFieldSchema::new(
							"days",
							ValueSchema::U64(default()),
						)],
					})),
				},
			],
		});
		let (mut world, root) =
			build(schema, value!({ "field": { "Snoozed": { "days": 2 } } }));
		test_ext::render_world(&mut world, root)
			.xpect_contains("<option value=\"Active\"")
			.xpect_contains("name=\"field.Snoozed.days\"");

		// choosing a variant writes that variant's zero, and the payload
		// controls follow the value rather than the choice
		let select = test_ext::element_in(&mut world, "select");
		*world.entity_mut(select).get_mut::<Value>().unwrap() =
			Value::str("Active");
		test_ext::settle_world(&mut world);
		world
			.entity(root)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "field": "Active" }));
		test_ext::render_world(&mut world, root)
			.xnot()
			.xpect_contains("field.Snoozed.days");
	}

	/// A field whose schema a sibling names gets the control that sibling's
	/// *value* asks for, which is the dependent arm made editable: the pair
	/// `{ schema, value }` renders a checkbox when the schema says `Bool`.
	#[beet_core::test]
	fn a_dependent_field_dispatches_on_its_sibling() {
		let pair = ValueSchema::Struct(StructSchema {
			name: Some("Pair".into()),
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("schema", ValueSchema::meta()),
				NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
			],
		});
		let (mut world, root) = build(
			pair,
			value!({ "field": { "schema": { "Bool": {} }, "value": true } }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.value\"")
			.xpect_contains("type=\"checkbox\"");
	}

	/// The whole form gathers on submit as a *typed* map: a checkbox submits a
	/// `Bool` and a number an `Int`, which is what lets a schema-driven
	/// submission validate against the schema that generated it.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn submit_gathers_typed_values() {
		let mut app = test_ext::form_app();
		let captured = Store::new(None::<Value>);
		app.world_mut().add_observer(move |ev: On<Submit>| {
			captured.set(Some(ev.values.clone()));
		});
		let root = app
			.world_mut()
			.spawn_template(rsx! {
				<div>
					<DynamicForm schema={ValueSchema::of::<Profile>()}>
						<Button>"Save"</Button>
					</DynamicForm>
				</div>
			})
			.unwrap()
			.id();
		app.world_mut().entity_mut(root).insert(Document::new(
			value!({ "name": "ada", "count": 7, "active": true }),
		));
		app.update();

		let button = test_ext::element(&mut app, "button");
		test_ext::click(&mut app, button);
		let values = captured.get().unwrap();
		values.get("name").unwrap().xpect_eq(Value::str("ada"));
		values.get("count").unwrap().xpect_eq(Value::Int(7));
		values.get("active").unwrap().xpect_eq(Value::Bool(true));
	}

	/// Typing into a generated control writes through its own extended path into
	/// the right document slot, on the terminal, driven by real key input.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn typing_reaches_a_nested_document_path() {
		let mut app = test_ext::form_app();
		let root = app
			.world_mut()
			.spawn_template(rsx! {
				<div>
					<DynamicForm
						schema={ValueSchema::of::<Profile>()}
						field={FieldRef::new("profile")}
					/>
				</div>
			})
			.unwrap()
			.id();
		app.world_mut()
			.entity_mut(root)
			.insert(Document::new(value!({
				"profile": { "name": "", "count": 0, "active": false }
			})));
		app.update();

		// the first input in document order is the `name` text field
		let (window, _) = test_ext::focus_element(&mut app, "input");
		test_ext::type_text(&mut app, window, "hi");

		app.world()
			.get::<Document>(root)
			.unwrap()
			.get_field::<String>(&[
				FieldSegment::key("profile"),
				FieldSegment::key("name"),
			])
			.unwrap()
			.xpect_eq("hi".to_string());
	}
}

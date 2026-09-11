//! The composite arms of a [`DynamicForm`](super::DynamicForm): the shapes that
//! recurse into a group of nested controls, each child's [`FieldRef`] extending
//! its parent's path.
//!
//! A struct and a tuple are decided by the schema alone, so their rows are built
//! once. A list and a map are not — no control types an item into existence — so
//! their rows ride a [`ValueRebuild`](super::value_rebuild::ValueRebuild) keyed
//! by position or by key, with the structural edits themselves owned by
//! [`collection_edit`](super::collection_edit).
use super::collection_edit::CollectionEdit;
use super::collection_edit::add_entry_row;
use super::collection_edit::edit_button;
use super::field_layout::child_field;
use super::field_layout::empty_note;
use super::field_layout::field_label;
use super::field_layout::group;
use super::field_layout::hinted;
use super::field_layout::labeled;
use super::form::schema_field;
use super::value_rebuild::RebuildKey;
use super::value_rebuild::ValueRebuild;
use crate::prelude::*;
use beet_core::prelude::*;

/// The struct arm: one nested control per named field, each [`FieldRef`]
/// extending this one's path and labelled by the field's label hint (else its
/// key). The form's own top level *is* the group, so only a nested struct wraps
/// its rows in a disclosure.
pub(super) fn struct_field<'a>(
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
		.map(|named| struct_row(resolver, named, field, depth))
		.collect()
}

/// One named field's control under its label and hint, its [`FieldRef`]
/// extending the struct's path.
pub(super) fn struct_row(
	resolver: SchemaResolver,
	named: &NamedFieldSchema,
	field: &FieldRef,
	depth: usize,
) -> Snippet {
	hinted(
		named.description.as_deref(),
		schema_field(
			resolver,
			&named.schema,
			child_field(field, named.key.clone()),
			Some(field_label(named)),
			depth + 1,
		),
	)
}

/// The title a nested struct's disclosure wears: its label hint, else the name
/// the schema declares for itself, else the path it binds.
pub(super) fn struct_title(
	schema: &StructSchema,
	field: &FieldRef,
	label: Option<String>,
) -> String {
	label
		.or_else(|| schema.name.as_ref().map(|name| name.to_string()))
		.unwrap_or_else(|| field.field_path.to_string())
}

/// The tuple arm: one control per element, labelled by the element's
/// description hint else its position. A tuple's arity is its schema's, so it
/// has no add or remove.
pub(super) fn tuple_field<'a>(
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
/// The rows ride a [`ValueRebuild`](super::value_rebuild::ValueRebuild) keyed
/// by *index*, so an append builds one row and a pop drops one while every
/// other row keeps its entity, and editing one is its own control's business.
pub(super) fn list_field(
	resolver: SchemaResolver,
	schema: &ListSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let (item, rows_field) = (schema.item.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		|value| match item_count(value) {
			0 => vec![RebuildKey::Empty],
			count => (0..count).map(RebuildKey::Index).collect(),
		},
		move |resolver, _value, key| match key {
			RebuildKey::Index(index) => {
				list_row(resolver, &item, &rows_field, *index, depth)
			}
			_ => empty_note("No items yet"),
		},
	);
	let zero = schema.item.default_value_in(resolver);
	let add = add_label(label.as_deref(), "item");
	group(label, rsx! {
		<div {(field.clone(), rebuild)}/>
		{edit_button(add, field, CollectionEdit::Push(zero))}
	})
}

/// What a collection's add button is called: `Add to constraints` where the
/// collection has a name of its own, else `Add item`/`Add entry`.
///
/// A form nests collections (a schema's fields, each field's constraints), so
/// several add buttons can share a screen; naming the collection is what says
/// which list a press grows. The unnamed case is the top level, where there is
/// only one collection and nothing to disambiguate.
pub(super) fn add_label(collection: Option<&str>, noun: &str) -> String {
	match collection {
		Some(name) => format!("Add to {name}"),
		None => format!("Add {noun}"),
	}
}

/// One list row: the item's own controls under an ordinal title, and the button
/// that drops it.
///
/// The ordinal is the label the item's own group wears, so a list of structs
/// reads `Item 1`, `Item 2` rather than repeating the item schema's type name
/// once per row, which named the *kind* and so distinguished nothing.
fn list_row(
	resolver: SchemaResolver,
	item: &ValueSchema,
	field: &FieldRef,
	index: usize,
	depth: usize,
) -> Snippet {
	rsx! {
		<div {row_card()}>
			{schema_field(
				resolver,
				item,
				child_field(field, index),
				Some(format!("Item {}", index + 1)),
				depth + 1,
			)}
			{edit_button(
				"Remove",
				field.clone(),
				CollectionEdit::Remove(FieldSegment::index(index)),
			)}
		</div>
	}
	.any_snippet()
}

/// A collection row's card, colocated with the row: an outlined panel laid out
/// as the same stretch column the shipped `<form>` rule is.
///
/// The outline is what makes the row's own `Remove` legibly *its*: the button
/// sits inside a visible boundary with the controls it drops, where an unbounded
/// row left it floating between two sets of fields belonging to neither.
///
/// The column stretches the item's controls to the row's width while `Remove`
/// keeps its own, which is the `form button` rule's `align-self` doing its job —
/// a flex-only property a block container would ignore, leaving the button a
/// full-width band.
fn row_card() -> impl Bundle {
	(Classes::new([classes::CARD_OUTLINED]), inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Vertical
		),
		(
			style::common_props::AlignItemsProp,
			style::AlignItems::Stretch
		),
	])
}

/// The map arm: one control per entry, labelled by its key, with a remove button
/// beside it and a key to type beside the add button.
///
/// The entries ride a [`ValueRebuild`](super::value_rebuild::ValueRebuild) keyed
/// by the map's *keys*, in sorted order so an added entry slots in beside its
/// neighbours while every other entry keeps its entity.
pub(super) fn map_field(
	resolver: SchemaResolver,
	schema: &MapSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let (map_schema, entries_field) = (schema.clone(), field.clone());
	let rebuild = ValueRebuild::new(
		|value| match entry_keys(value) {
			keys if keys.is_empty() => vec![RebuildKey::Empty],
			keys => keys.into_iter().map(RebuildKey::Name).collect(),
		},
		// an entry is typed by the map, which for a keyed map means its key
		move |resolver, _value, key| match key {
			RebuildKey::Name(key) => {
				match map_schema.entry_schema(resolver, key) {
					Ok(schema) => map_entry(
						resolver,
						schema,
						&entries_field,
						key.clone(),
						depth,
					),
					Err(err) => empty_note(format!("{key}: {err}")),
				}
			}
			_ => empty_note("No entries yet"),
		},
	);
	// a keyed map's zero depends on the key the picker will choose
	let zero = match schema {
		MapSchema::Uniform { value } => value.default_value_in(resolver),
		MapSchema::Keyed => Value::Null,
	};
	let add = add_label(label.as_deref(), "entry");
	group(label, rsx! {
		<div {(field.clone(), rebuild)}/>
		{add_entry_row(field, zero, add)}
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
		<div {row_card()}>
			{schema_field(
				resolver,
				schema,
				child_field(field, key.clone()),
				Some(key.to_string()),
				depth + 1,
			)}
			{edit_button(
				"Remove",
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

#[cfg(test)]
mod test {
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

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
		let html = test_ext::form_html(ValueSchema::Struct(StructSchema {
			name: Some("Outer".into()),
			description: None,
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"profile",
				ValueSchema::of::<Profile>(),
			)],
		}));
		html.clone()
			// the top-level field is a row, its nested struct a titled section
			.xpect_contains("<section>")
			.xpect_contains(">Profile</h3>")
			.xpect_contains("name")
			.xpect_contains("count")
			.xpect_contains("type=\"text\"")
			.xpect_contains("type=\"number\"")
			.xpect_contains("type=\"checkbox\"");
		// the nested paths are the leaves' names, so a submit gathers them whole
		html.clone().xpect_contains("name=\"field.profile.name\"");
		// ...and the top level is the form itself, not a group inside it
		html.xnot().xpect_contains(">Outer</h3>");
	}

	/// A field's label hint replaces its key as the visible label; the key is
	/// still what the path (and so the binding) uses.
	#[beet_core::test]
	fn a_label_hint_replaces_the_key() {
		test_ext::form_html(ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("name", ValueSchema::String(default()))
					.with_label("Display Name"),
			],
		}))
		.xpect_contains("Display Name")
		.xpect_contains("name=\"field.name\"");
	}

	/// A list's items each get the control their item schema asks for, bound to
	/// their own index, so an edit reaches one row and no other.
	#[beet_core::test]
	fn a_list_generates_a_control_per_item() {
		let (mut world, root) = test_ext::build_form(
			ValueSchema::of::<Vec<String>>(),
			"field",
			value!({ "field": ["buy milk", "walk dog"] }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.[0]\"")
			.xpect_contains("name=\"field.[1]\"");

		let input = test_ext::elements_in(&mut world, "input")[1];
		*world.entity_mut(input).get_mut::<Value>().unwrap() =
			Value::str("walk cat");
		test_ext::settle_world(&mut world);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "field": ["buy milk", "walk cat"] }));
	}

	/// An append reconciles the rows by index: the existing row's control and
	/// the add button keep their entities, so focus in either survives the
	/// new row, and only the new row's control is built.
	#[beet_core::test]
	fn an_append_keeps_the_existing_rows() {
		let (mut world, _) = test_ext::build_form(
			ValueSchema::of::<Vec<String>>(),
			"field",
			value!({ "field": ["buy milk"] }),
		);
		let inputs = test_ext::elements_in(&mut world, "input");
		let add = test_ext::collection_add(&mut world, "field");
		test_ext::click_world(&mut world, add);
		let after = test_ext::elements_in(&mut world, "input");
		after.len().xpect_eq(2);
		after[0].xpect_eq(inputs[0]);
		test_ext::collection_add(&mut world, "field").xpect_eq(add);
	}

	/// A list of structs is a control per field per row, so the todo app's rows
	/// are editable rather than a read-only table.
	#[beet_core::test]
	fn a_list_of_structs_recurses_per_row() {
		let (mut world, root) = test_ext::build_form(
			ValueSchema::of::<Vec<Profile>>(),
			"field",
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
		let (mut world, root) = test_ext::build_form(
			ValueSchema::of::<(String, i64)>(),
			"field",
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
		let (mut world, root) = test_ext::build_form(
			ValueSchema::Map(MapSchema::uniform(ValueSchema::Bool(default()))),
			"field",
			value!({ "field": { "done": true, "urgent": false } }),
		);
		test_ext::render_world(&mut world, root)
			.xpect_contains("name=\"field.done\"")
			.xpect_contains("name=\"field.urgent\"");
	}
}

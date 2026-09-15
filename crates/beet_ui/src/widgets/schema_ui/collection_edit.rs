//! The add and remove controls of a [`DynamicForm`](super::DynamicForm)'s list
//! and map arms.
//!
//! A collection's *contents* are not a leaf: no control can type an item into
//! existence, so the form emits a button per structural edit and lets the
//! generated controls edit the items themselves. Each button applies exactly one
//! edit to exactly one bound field, so the whole widget still binds one
//! `(document, field path)` and writes only through it.
//!
//! Every button is a `<button type="button">`: the browser's rule that an action
//! button is not a submit, which is what lets a list live inside a form without
//! committing it on every row (`fire_form_submit`).
use super::component_picker::ComponentPicker;
use super::field_layout::labeled;
use crate::prelude::Button;
use crate::prelude::*;
use beet_core::prelude::*;

/// One structural edit to a collection field, the whole vocabulary of what a
/// generated add or remove button does.
///
/// A list and a map differ in how a position is *named*, not in what removing
/// one means, so both removes are one arm over the [`FieldSegment`] the document
/// layer already calls a position. Adding is the arm that genuinely differs:
/// appending needs nothing, and an entry needs a name first.
#[derive(Debug, Clone)]
pub(in crate::widgets) enum CollectionEdit {
	/// Append the item schema's zero to the list.
	Push(Value),
	/// Insert the value schema's zero under the key typed into the sibling
	/// [`NewEntryKey`] input, the one edit with a precondition.
	Insert(Value),
	/// Insert, under the key the sibling [`NewEntryKey`] picker chose, the zero
	/// of the schema that key names ([`MapSchema::entry_schema`]), resolved at
	/// the press since the key decides it. An entity reference inside it
	/// starts at the first entity the relation may target, so an added
	/// `ChildOf` is valid before its target is chosen.
	InsertKeyed,
	/// Drop the item or entry at this position.
	Remove(FieldSegment),
}

/// A button applying one [`CollectionEdit`] to the field it names.
///
/// The field is the *collection's* path, not the item's, so an edit at a
/// position is resolved against the whole collection in one write.
#[derive(Component)]
#[component(on_add = hook_ext::observe(apply_collection_edit))]
pub(in crate::widgets) struct CollectionButton {
	pub(in crate::widgets) field: FieldRef,
	pub(in crate::widgets) edit: CollectionEdit,
}

/// Marks the key control of a map control's add-entry row: an unbound text
/// field (or, for a keyed map, the [`ComponentPicker`] select) whose local
/// [`Value`] is the key its sibling button inserts under, the
/// [`Action`](WritePolicy::Action) it writes by.
///
/// Deliberately unnamed, so a key still being typed is not gathered into the
/// form's submission alongside the entries it has yet to create.
#[derive(Component)]
#[require(WritePolicy = WritePolicy::Action)]
pub(in crate::widgets) struct NewEntryKey;

/// A `<button type="button">` applying `edit` to `field` on activation.
///
/// Tonal rather than text: a structural edit is the only thing on a generated
/// form that is not a labelled control, and a text button paints in the same
/// `OnSurface` ink as the labels around it, so in the terminal an `add` reads
/// as a word rather than as something to press. A tonal fill is the lightest
/// variant that still carries its own container, and unlike an outlined one it
/// stays a single row.
pub(super) fn edit_button(
	label: impl Into<String>,
	field: FieldRef,
	edit: CollectionEdit,
) -> Snippet {
	let label = label.into();
	rsx! {
		<Button
			action=true
			variant={ButtonVariant::Tonal}
			{CollectionButton { field, edit }}
		>{label}</Button>
	}
	.any_snippet()
}

/// The add-entry row of a map control: a key to type and the button that
/// inserts it, holding the zero every new entry starts as.
pub(super) fn add_entry_row(
	field: FieldRef,
	zero: Value,
	label: String,
) -> Snippet {
	rsx! {
		<div>
			// labelled, not merely placeheld: the charcell renderer paints no
			// placeholder, so on a terminal this is otherwise an unexplained
			// empty box beside a button
			{labeled(
				Some("New key".into()),
				rsx! { <TextField {NewEntryKey} placeholder="key"/> },
			)}
			{edit_button(label, field, CollectionEdit::Insert(zero))}
		</div>
	}
	.any_snippet()
}

/// The add-entry row of a keyed map: a [`ComponentPicker`] over the registry
/// and the button that inserts the chosen component's zero.
pub(super) fn add_component_row(field: FieldRef, label: String) -> Snippet {
	rsx! {
		<div>
			{labeled(
				Some("Component".into()),
				rsx! { <ComponentPicker field={field.clone()}/> },
			)}
			{edit_button(label, field, CollectionEdit::InsertKeyed)}
		</div>
	}
	.any_snippet()
}

/// Observer: activating a collection button applies its edit to the bound
/// field, which the document sync then carries into every generated control.
fn apply_collection_edit(
	ev: On<PointerUp>,
	buttons: Query<&CollectionButton>,
	elements: ElementQuery,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
	new_keys: Query<(), With<NewEntryKey>>,
	values: Query<&Value>,
	schemas: Option<Res<SchemaRegistry>>,
	types: Option<Res<AppTypeRegistry>>,
	mut docs: DocumentQuery,
	mut commands: Commands,
) -> Result {
	// the event bubbles; act only at the button carrying the edit
	let entity = ev.event_target();
	let Ok(button) = buttons.get(entity) else {
		return OK;
	};
	// an insert is keyed by the control beside it, and nothing chosen is nothing
	// to do, exactly as an empty form field is
	let key_input = new_entry_key(entity, &children, &parents, &new_keys);
	let key = key_input
		.and_then(|input| entry_key(&elements, &values, input))
		.filter(|key| !key.is_empty());
	let edit = match (&button.edit, key) {
		(CollectionEdit::Insert(_) | CollectionEdit::InsertKeyed, None) => {
			return OK;
		}
		(CollectionEdit::Insert(zero), Some(key)) => {
			ResolvedEdit::Insert(key, zero.clone())
		}
		(CollectionEdit::InsertKeyed, Some(key)) => {
			let types = types.as_ref().map(|types| types.read());
			let resolver =
				super::resolver(schemas.as_deref(), types.as_deref());
			let scene =
				docs.field_value(entity, &whole_document(&button.field))?;
			ResolvedEdit::Insert(
				key.clone(),
				keyed_zero(resolver, &scene, &button.field, &key)?,
			)
		}
		(CollectionEdit::Push(item), _) => ResolvedEdit::Push(item.clone()),
		(CollectionEdit::Remove(segment), _) => {
			ResolvedEdit::Remove(segment.clone())
		}
	};
	docs.with_field(entity, &button.field, move |value| -> Result {
		match edit {
			ResolvedEdit::Push(item) => value.as_list_mut_or_init()?.push(item),
			// a key already present is left alone: the picker offers what is
			// missing, and re-inserting would reset the entry to its zero
			ResolvedEdit::Insert(key, zero) => {
				let map = as_map_mut_or_init(value)?;
				if !map.contains(&key) {
					map.insert(key, zero);
				}
			}
			ResolvedEdit::Remove(FieldSegment::ArrayIndex(index)) => {
				let list = value.as_list_mut_or_init()?;
				if index < list.len() {
					list.remove(index);
				}
			}
			ResolvedEdit::Remove(FieldSegment::ObjectKey(key)) => {
				as_map_mut_or_init(value)?.remove(key.as_str());
			}
		}
		OK
	})??;
	// the key is spent, so the next entry starts empty
	if let Some(input) = key_input {
		commands.entity(input).insert(Value::str(""));
	}
	OK
}

/// A [`CollectionEdit`] with its key and zero resolved, ready to apply.
enum ResolvedEdit {
	Push(Value),
	Insert(SmolStr, Value),
	Remove(FieldSegment),
}

/// The key the add row's control holds: a text field's trimmed text, or for a
/// picker `<select>` its chosen option, falling back to its first one as a
/// browser does for an untouched select.
fn entry_key(
	elements: &ElementQuery,
	values: &Query<&Value>,
	input: Entity,
) -> Option<SmolStr> {
	let chosen = values
		.get(input)
		.ok()
		.and_then(|value| value.as_str().ok())
		.map(|key| SmolStr::from(key.trim()))
		.filter(|key| !key.is_empty());
	let view = elements.get(input).ok()?;
	match (view.tag(), chosen) {
		("select", None) => elements
			.iter_descendants_inclusive(input)
			.find(|child| child.tag() == "option")
			.map(|option| SmolStr::from(option.option_value())),
		(_, chosen) => chosen,
	}
}

/// The whole document `field` binds into, ie the scene an added reference is
/// seeded from.
fn whole_document(field: &FieldRef) -> FieldRef {
	FieldRef {
		document: field.document.clone(),
		field_path: FieldPath::default(),
		on_missing: default(),
	}
}

/// The zero a keyed entry starts as: the schema `key` names, with every entity
/// reference in it pointing at the first entity the relation may target, so
/// the document layer accepts the add before a picker chooses.
fn keyed_zero(
	resolver: SchemaResolver,
	scene: &Value,
	field: &FieldRef,
	key: &str,
) -> Result<Value> {
	let schema = MapSchema::Keyed.entry_schema(resolver, key)?;
	let mut zero = schema.default_value_in(resolver);
	let candidate = match (
		SceneEntities::of(scene),
		resolver.types(),
		SceneEntities::position(&field.field_path.with_pushed(key)),
	) {
		(Ok(entities), Some(types), Some((source, relation))) => entities
			.candidates(types, relation, source)?
			.first()
			.map(|target| EntitySchema::reference(*target))
			.transpose()?,
		_ => None,
	};
	if let Some(reference) = candidate {
		seed_references(resolver, &schema, &mut zero, &reference);
	}
	Ok(zero)
}

/// Point every `Entity` leaf of `value`, as `schema` describes it, at
/// `reference`: a zero's null references made valid.
fn seed_references(
	resolver: SchemaResolver,
	schema: &ValueSchema,
	value: &mut Value,
	reference: &Value,
) {
	match (schema, value) {
		(ValueSchema::Entity(_), value) => *value = reference.clone(),
		(ValueSchema::Optional(inner), value) => {
			seed_references(resolver, inner, value, reference)
		}
		(ValueSchema::Struct(schema), Value::Map(map)) => {
			for field in &schema.fields {
				if let Some(value) = map.0.get_mut(field.key.as_str()) {
					seed_references(resolver, &field.schema, value, reference);
				}
			}
		}
		(ValueSchema::Tuple(schema), Value::List(items)) => {
			for (field, value) in schema.fields.iter().zip(items.iter_mut()) {
				seed_references(resolver, &field.schema, value, reference);
			}
		}
		(ValueSchema::Ref(schema_ref), value) => {
			if let Some(schema) = resolver.follow(schema_ref) {
				seed_references(resolver, schema, value, reference);
			}
		}
		_ => {}
	}
}

/// The [`NewEntryKey`] input sharing a parent with `button`, ie the key half of
/// the add-entry row it is the button half of.
fn new_entry_key(
	button: Entity,
	children: &Query<&Children>,
	parents: &Query<&ChildOf>,
	new_keys: &Query<(), With<NewEntryKey>>,
) -> Option<Entity> {
	// a descendant walk of the add row rather than a scan of the button's own
	// siblings: the input wears a `<label>` (its key is not self-evident on a
	// terminal, which paints no placeholder), so it sits a level deeper than the
	// button it belongs to.
	let mut stack = vec![parents.get(button).ok()?.parent()];
	while let Some(entity) = stack.pop() {
		if new_keys.contains(entity) {
			return Some(entity);
		}
		if let Ok(kids) = children.get(entity) {
			stack.extend(kids.iter());
		}
	}
	None
}

/// A map field's entries, coercing a missing or null field into an empty map
/// first, exactly as [`Value::as_list_mut_or_init`] does for a list.
fn as_map_mut_or_init(value: &mut Value) -> Result<&mut Map> {
	if value.is_null() {
		*value = Value::map();
	}
	value.as_map_mut()?.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	/// A form over `schema` bound to an `"items"` field, settled.
	fn build(schema: ValueSchema, document: Value) -> (World, Entity) {
		test_ext::build_form(schema, "items", document)
	}

	/// The generated buttons in document order: one remove per row, then the add.
	fn buttons(world: &mut World) -> Vec<Entity> {
		test_ext::elements_in(world, "button")
	}

	/// The add button appends the item schema's zero, so a fresh row arrives
	/// already valid rather than as a null the schema it came from rejects.
	#[beet_core::test]
	fn adding_appends_the_item_zero() {
		let (mut world, root) =
			build(ValueSchema::of::<Vec<String>>(), value!({ "items": [] }));
		let add = *buttons(&mut world).last().unwrap();
		test_ext::click_world(&mut world, add);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "items": [""] }));
	}

	/// A remove button drops its own row and nothing else, so the control edits
	/// the list it reads rather than a copy of it.
	#[beet_core::test]
	fn removing_drops_its_own_row() {
		let (mut world, root) = build(
			ValueSchema::of::<Vec<String>>(),
			value!({ "items": ["a", "b", "c"] }),
		);
		let remove = buttons(&mut world)[1];
		test_ext::click_world(&mut world, remove);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "items": ["a", "c"] }));
	}

	/// A collection control inside a form never submits it: its buttons are
	/// `type="button"`, the browser's own rule.
	#[beet_core::test]
	fn a_collection_button_is_not_a_submit() {
		let (mut world, _) =
			build(ValueSchema::of::<Vec<String>>(), value!({ "items": [] }));
		let submitted = Store::new(false);
		let captured = submitted.clone();
		world.add_observer(move |_: On<Submit>| captured.set(true));
		let add = *buttons(&mut world).last().unwrap();
		test_ext::click_world(&mut world, add);
		submitted.get().xpect_false();
	}

	/// A map schema, whose entries are keyed by hand rather than appended.
	fn map_schema() -> ValueSchema {
		ValueSchema::Map(MapSchema::uniform(ValueSchema::Bool(default())))
	}

	/// A map entry is added under the key typed beside the button, which is
	/// cleared once spent, and dropped by its own row's button.
	#[beet_core::test]
	fn a_map_entry_is_keyed_by_its_input() {
		let (mut world, root) = build(map_schema(), value!({ "items": {} }));
		let key_input = test_ext::element_in(&mut world, "input");
		world
			.entity_mut(key_input)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("done"));
		let add = *buttons(&mut world).last().unwrap();
		test_ext::click_world(&mut world, add);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "items": { "done": false } }));
		// the spent key is cleared, so the next entry starts empty
		world
			.entity(key_input)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str(""));

		// the entry's own remove button drops it
		let remove = buttons(&mut world)[0];
		test_ext::click_world(&mut world, remove);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "items": {} }));
	}

	/// An add with nothing typed does nothing, rather than creating an entry
	/// under the empty key.
	#[beet_core::test]
	fn an_unkeyed_entry_is_not_added() {
		let (mut world, root) = build(map_schema(), value!({ "items": {} }));
		let add = *buttons(&mut world).last().unwrap();
		test_ext::click_world(&mut world, add);
		test_ext::document_of(&mut world, root)
			.xpect_eq(value!({ "items": {} }));
	}
}

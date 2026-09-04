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
pub(super) enum CollectionEdit {
	/// Append the item schema's zero to the list.
	Push(Value),
	/// Insert the value schema's zero under the key typed into the sibling
	/// [`NewEntryKey`] input, the one edit with a precondition.
	Insert(Value),
	/// Drop the item or entry at this position.
	Remove(FieldSegment),
}

/// A button applying one [`CollectionEdit`] to the field it names.
///
/// The field is the *collection's* path, not the item's, so an edit at a
/// position is resolved against the whole collection in one write.
#[derive(Component)]
#[component(on_add = hook_ext::observe(apply_collection_edit))]
pub(super) struct CollectionButton {
	pub(super) field: FieldRef,
	pub(super) edit: CollectionEdit,
}

/// Marks the key input of a map control's add-entry row: an unbound text field
/// whose local [`Value`] is the key its sibling button inserts under.
///
/// Deliberately unnamed, so a key still being typed is not gathered into the
/// form's submission alongside the entries it has yet to create.
#[derive(Component)]
pub(super) struct NewEntryKey;

/// A `<button type="button">` applying `edit` to `field` on activation.
pub(super) fn edit_button(
	label: impl Into<String>,
	field: FieldRef,
	edit: CollectionEdit,
) -> Snippet {
	let label = label.into();
	rsx! {
		<Button
			action=true
			variant={ButtonVariant::Text}
			{CollectionButton { field, edit }}
		>{label}</Button>
	}
	.any_snippet()
}

/// The add-entry row of a map control: a key to type and the button that
/// inserts it, holding the zero every new entry starts as.
pub(super) fn add_entry_row(field: FieldRef, zero: Value) -> Snippet {
	rsx! {
		<div>
			<TextField {NewEntryKey} placeholder="key"/>
			{edit_button("add", field, CollectionEdit::Insert(zero))}
		</div>
	}
	.any_snippet()
}

/// Observer: activating a collection button applies its edit to the bound
/// field, which the document sync then carries into every generated control.
fn apply_collection_edit(
	ev: On<PointerUp>,
	buttons: Query<&CollectionButton>,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
	new_keys: Query<(), With<NewEntryKey>>,
	mut values: Query<&mut Value>,
	mut docs: DocumentQuery,
) -> Result {
	// the event bubbles; act only at the button carrying the edit
	let entity = ev.event_target();
	let Ok(button) = buttons.get(entity) else {
		return OK;
	};
	// an insert is keyed by the input beside it, and nothing typed is nothing to
	// do, exactly as an empty form field is
	let key_input = new_entry_key(entity, &children, &parents, &new_keys);
	let key = key_input
		.and_then(|input| values.get(input).ok())
		.and_then(|value| value.as_str().ok())
		.map(|key| SmolStr::from(key.trim()))
		.filter(|key| !key.is_empty());
	if matches!(button.edit, CollectionEdit::Insert(_)) && key.is_none() {
		return OK;
	}
	let edit = button.edit.clone();
	docs.with_field(entity, &button.field, move |value| -> Result {
		match edit {
			CollectionEdit::Push(item) => {
				value.as_list_mut_or_init()?.push(item)
			}
			CollectionEdit::Insert(zero) => {
				let key =
					key.ok_or_else(|| bevyhow!("a map entry needs a key"))?;
				as_map_mut_or_init(value)?.insert(key, zero);
			}
			CollectionEdit::Remove(FieldSegment::ArrayIndex(index)) => {
				let list = value.as_list_mut_or_init()?;
				if index < list.len() {
					list.remove(index);
				}
			}
			CollectionEdit::Remove(FieldSegment::ObjectKey(key)) => {
				as_map_mut_or_init(value)?.remove(key.as_str());
			}
		}
		OK
	})??;
	// the key is spent, so the next entry starts empty
	if let Some(input) = key_input {
		values.get_mut(input)?.set_if_neq(Value::str(""));
	}
	OK
}

/// The [`NewEntryKey`] input sharing a parent with `button`, ie the key half of
/// the add-entry row it is the button half of.
fn new_entry_key(
	button: Entity,
	children: &Query<&Children>,
	parents: &Query<&ChildOf>,
	new_keys: &Query<(), With<NewEntryKey>>,
) -> Option<Entity> {
	children
		.get(parents.get(button).ok()?.parent())
		.ok()?
		.iter()
		.find(|sibling| new_keys.contains(*sibling))
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
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A form over `schema` bound to an `"items"` field, settled.
	fn build(schema: ValueSchema, document: Value) -> (World, Entity) {
		let mut world = test_ext::form_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicForm schema={schema} field={FieldRef::new("items")}/>
				</div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(document));
		test_ext::settle_world(&mut world);
		(world, root)
	}

	fn document(world: &mut World, root: Entity) -> Value {
		world.entity(root).get::<Document>().unwrap().0.clone()
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
		document(&mut world, root).xpect_eq(value!({ "items": [""] }));
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
		document(&mut world, root).xpect_eq(value!({ "items": ["a", "c"] }));
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
		ValueSchema::Map(MapSchema {
			value: Box::new(ValueSchema::Bool(default())),
		})
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
		document(&mut world, root)
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
		document(&mut world, root).xpect_eq(value!({ "items": {} }));
	}

	/// An add with nothing typed does nothing, rather than creating an entry
	/// under the empty key.
	#[beet_core::test]
	fn an_unkeyed_entry_is_not_added() {
		let (mut world, root) = build(map_schema(), value!({ "items": {} }));
		let add = *buttons(&mut world).last().unwrap();
		test_ext::click_world(&mut world, add);
		document(&mut world, root).xpect_eq(value!({ "items": {} }));
	}
}

//! The `Entity` arm of a [`DynamicForm`](super::DynamicForm): a picker over
//! the entities of the scene document the field binds into.
//!
//! An entity reference is a file key ([`EntitySchema::reference`]), which no
//! typed control can produce, so the arm is a [`Select`] whose options are the
//! scene's entities, labelled as the tree labels them
//! ([`SceneEntities::label`]) and filtered by the [`RelationMeta`] of the
//! relation the field sits in: a `ChildOf` picker never offers the entity's
//! own subtree, since the document layer would refuse the cycle.
//!
//! The options ride a [`ValueRebuild`] over the *whole* document, keyed on the
//! candidates, so an entity added or renamed elsewhere reaches the picker. The
//! select's own [`Value`] is the chosen key, a projection of the field it
//! edits: written through as a reference by [`write_picked_entity`], exactly
//! as the variant select writes its variant, and read back from the document
//! by [`follow_picked_entity`], so a target changed from outside shows without
//! the select being rebuilt under the focus that chose it.
use super::field_layout::empty_note;
use super::field_layout::labeled;
use super::value_rebuild::RebuildKey;
use super::value_rebuild::ValueRebuild;
use crate::prelude::*;
use beet_core::prelude::*;

/// Marks the `<select>` choosing the target of an `Entity` field, whose own
/// [`Value`] is the target's file key as text rather than the field's value.
///
/// Scoped to the widget set because [`write_picked_entity`] is registered by
/// [`FormPlugin`](crate::prelude::FormPlugin), as the variant select's is.
#[derive(Component)]
pub(in crate::widgets) struct EntityPicker {
	/// The reference field this picker chooses the target of.
	pub(super) field: FieldRef,
}

/// The entity arm: the picker under its label, riding a rebuild over the
/// scene it picks from.
pub(super) fn entity_field(field: FieldRef, label: Option<String>) -> Snippet {
	// the holder binds the whole document, since the candidates are every
	// entity the scene holds, not the field
	let scene_field = FieldRef {
		document: field.document.clone(),
		field_path: FieldPath::default(),
		on_missing: default(),
	};
	let (keyed, built) = (field.clone(), field);
	let rebuild = ValueRebuild::new(
		move |scene| vec![RebuildKey::Name(fingerprint(scene, &keyed))],
		move |resolver, scene, _key| picker(resolver, scene, &built),
	);
	labeled(label, rsx! { <div {(scene_field, rebuild)}/> })
}

/// What one generation of the picker offers: the candidates and their labels,
/// any change to which rebuilds it. The current target is not part of it,
/// since the select follows that without being rebuilt.
fn fingerprint(scene: &Value, field: &FieldRef) -> SmolStr {
	let Ok(entities) = SceneEntities::of(scene) else {
		return SmolStr::default();
	};
	// a field into no scene position has every entity as a candidate, so the
	// relation only narrows the fingerprint where it narrows the options
	let labels = entities
		.keys()
		.unwrap_or_default()
		.into_iter()
		.map(|key| entities.label(key))
		.collect::<Vec<_>>();
	format!(
		"{:?}|{labels:?}",
		SceneEntities::position(&field.field_path)
	)
	.into()
}

/// The file key the field currently references, if any.
fn current(scene: &Value, field: &FieldRef) -> Option<u32> {
	scene
		.get_path(&field.field_path)
		.and_then(EntitySchema::file_key)
}

/// One generation: the `<select>` over the entities the field may reference,
/// seeded with the one it does.
fn picker(
	resolver: SchemaResolver,
	scene: &Value,
	field: &FieldRef,
) -> Snippet {
	let Ok(entities) = SceneEntities::of(scene) else {
		return empty_note("No scene to pick an entity from");
	};
	// the relation's meta filters the candidates, where the field sits in one
	// and the world can say what it may do
	let candidates =
		match (resolver.types(), SceneEntities::position(&field.field_path)) {
			(Some(types), Some((source, relation))) => {
				entities.candidates(types, relation, source)
			}
			_ => entities.keys(),
		}
		.unwrap_or_default();
	let options = candidates
		.into_iter()
		.map(|key| {
			let label = entities.label(key);
			rsx! { <option value=key.to_string()>{label}</option> }
				.any_snippet()
		})
		.collect::<Vec<_>>();
	let name = field.field_path.to_string();
	let picker = EntityPicker {
		field: field.clone(),
	};
	// the local value is the key, seeded from what the field references, so a
	// select rendered for a value never asks to change it
	let value = Value::Str(
		current(scene, field)
			.map(|key| key.to_string())
			.unwrap_or_default()
			.into(),
	);
	rsx! {
		<Select name={name} {(picker, value)}>{options}</Select>
	}
	.any_snippet()
}

/// System: a picker shows the entity its field references, however the field
/// came to reference it: the read direction of the pair, run after
/// [`write_picked_entity`] so a choice just written reads back as itself and
/// an edit refused by the document layer (a cycle the picker could not see)
/// snaps the select back.
pub(in crate::widgets) fn follow_picked_entity(
	mut pickers: Query<(Entity, &EntityPicker, &mut Value)>,
	mut docs: DocumentQuery,
) {
	for (entity, picker, mut value) in pickers.iter_mut() {
		let Some(key) = docs
			.field_value(entity, &picker.field)
			.ok()
			.and_then(|value| EntitySchema::file_key(&value))
		else {
			continue;
		};
		value.set_if_neq(Value::str(key.to_string()));
	}
}

/// System: a chosen entity becomes a reference to it in the bound field.
///
/// Equality-guarded like every other sync: a picker showing what the field
/// already references is reporting, not asking, so only a *changed* key
/// writes. Registered by [`FormPlugin`](crate::prelude::FormPlugin).
pub(in crate::widgets) fn write_picked_entity(
	pickers: Populated<(Entity, &EntityPicker, &Value), Changed<Value>>,
	mut docs: DocumentQuery,
) -> Result {
	for (entity, picker, value) in pickers.iter() {
		let Some(key) =
			value.as_str().ok().and_then(|key| key.parse::<u32>().ok())
		else {
			continue;
		};
		let current = docs
			.field_value(entity, &picker.field)
			.ok()
			.and_then(|value| EntitySchema::file_key(&value));
		if current == Some(key) {
			continue;
		}
		let reference = EntitySchema::reference(key)?;
		docs.with_field(entity, &picker.field, move |slot| *slot = reference)?;
	}
	OK
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";

	/// `root { a, b }` as a scene document, with `b` the entity under edit.
	fn scene() -> Value {
		let child = |parent: u32| {
			value!({ "components": {
				"bevy_ecs::hierarchy::ChildOf": (EntitySchema::reference(parent).unwrap())
			} })
		};
		value!({
			"resources": {},
			"entities": {
				"0": { "components": { "bevy_ecs::name::Name": "root" } },
				"1": (child(0)),
				"2": (child(0))
			}
		})
	}

	/// A form over entity `2`'s `ChildOf`, the reparent control.
	fn build() -> (World, Entity) {
		let mut world = test_ext::form_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicForm
						schema={ValueSchema::Entity(default())}
						field={FieldRef::new(FieldPath::parse(
							"entities.2.components.bevy_ecs::hierarchy::ChildOf"
						))}
					/>
				</div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(scene()));
		test_ext::settle_world(&mut world);
		(world, root)
	}

	/// The picker offers every entity but the one being reparented, labelled
	/// by name or key and tag, and shows the current parent.
	#[beet_core::test]
	fn offers_the_candidates_and_shows_the_target() {
		let (mut world, root) = build();
		let html = test_ext::render_world(&mut world, root);
		html.clone()
			.xpect_contains("<option value=\"0\">root</option>")
			.xpect_contains("<option value=\"1\">#1</option>");
		html.xnot().xpect_contains("<option value=\"2\"");
		let select = test_ext::element_in(&mut world, "select");
		world
			.entity(select)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str("0"));
	}

	/// Choosing an entity writes a reference to it into the field, and nothing
	/// else in the document moves.
	#[beet_core::test]
	fn choosing_writes_a_reference() {
		let (mut world, root) = build();
		let select = test_ext::element_in(&mut world, "select");
		world
			.entity_mut(select)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("1"));
		test_ext::settle_world(&mut world);
		let scene = test_ext::document_of(&mut world, root);
		SceneEntities::of(&scene)
			.unwrap()
			.target(2, CHILD_OF)
			.unwrap()
			.xpect_eq(1);
		SceneEntities::of(&scene)
			.unwrap()
			.target(1, CHILD_OF)
			.unwrap()
			.xpect_eq(0);
	}

	/// An entity added elsewhere reaches the picker, and a target changed from
	/// outside shows in the same select: the options follow the document and
	/// the choice follows the field, neither rebuilding the control.
	#[beet_core::test]
	fn follows_the_document() {
		let (mut world, root) = build();
		{
			let mut document = world.entity_mut(root);
			let mut document = document.get_mut::<Document>().unwrap();
			document
				.0
				.get_mut("entities")
				.unwrap()
				.insert(
					"3",
					value!({ "components": { "bevy_ecs::name::Name": "c" } }),
				)
				.unwrap();
			*document
				.0
				.get_mut("entities")
				.unwrap()
				.get_mut("2")
				.unwrap()
				.get_mut("components")
				.unwrap()
				.get_mut(CHILD_OF)
				.unwrap() = EntitySchema::reference(1).unwrap();
		}
		test_ext::settle_world(&mut world);
		test_ext::render_world(&mut world, root)
			.xpect_contains("<option value=\"3\">c</option>");
		// the options rebuilt (an entity arrived), so the select is new; its
		// choice followed the field
		let select = test_ext::element_in(&mut world, "select");
		world
			.entity(select)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str("1"));

		// a target changed alone leaves the select in place and moves its choice
		*world
			.entity_mut(root)
			.get_mut::<Document>()
			.unwrap()
			.0
			.get_mut("entities")
			.unwrap()
			.get_mut("2")
			.unwrap()
			.get_mut("components")
			.unwrap()
			.get_mut(CHILD_OF)
			.unwrap() = EntitySchema::reference(3).unwrap();
		test_ext::settle_world(&mut world);
		test_ext::element_in(&mut world, "select").xpect_eq(select);
		world
			.entity(select)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str("3"));
	}

	/// Choosing keeps the select: the control that took the choice is the one
	/// still focused afterwards, and it shows what it chose.
	#[beet_core::test]
	fn choosing_keeps_the_select() {
		let (mut world, _) = build();
		let select = test_ext::element_in(&mut world, "select");
		world
			.entity_mut(select)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("1"));
		test_ext::settle_world(&mut world);
		test_ext::element_in(&mut world, "select").xpect_eq(select);
		world
			.entity(select)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str("1"));
	}
}

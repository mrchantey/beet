//! [`ValueRebuild`]: reconciling a value-generated subtree when the shape of
//! the value it was generated from changes.
//!
//! [`SchemaRebuild`](super::schema_rebuild::SchemaRebuild)'s twin, and the third
//! grain of a schema-driven widget's reactivity. A leaf's *value* rides its own
//! binding and a subtree's *schema* rides the registry; what neither covers is a
//! control whose very shape is decided by the value it edits: a list's rows, a
//! map's entries, an enum's payload, and a field whose schema a sibling names
//! ([`SchemaRef::AtField`]).
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Holds the builder of a subtree the bound [`Value`]'s shape decides, keyed so
/// a shape change reconciles the current generation rather than replacing it.
///
/// Rides a co-located [`FieldRef`] exactly as [`ReactiveChildren`] does: the ref
/// syncs the bound value onto this entity, and a `Changed<Value>` drives the
/// reconcile. The holder must therefore be an **element**, since a `Value` on a
/// tag-less node renders as text; its children are one generation and nothing
/// else, so a child under no [`RebuildKey`] is despawned as a vanished one.
///
/// The value decides the generation as a list of [`RebuildKey`]s, and a child
/// whose key survives a change is reused, entity and all, so a control keeps
/// its focus and caret while a row is appended beside it. Only a new key is
/// built and only a vanished one despawned; a change that leaves the keys alone
/// (a leaf edit inside the generation) builds nothing. Which part of the value
/// is a key is each arm's to say: a list's rows by index, a map's entries by
/// key, an enum's controls by variant, a dependent struct's rows by the schema
/// their sibling named.
///
/// The build closure is handed a live [`SchemaResolver`], because a child is
/// built long after the walk that authored it: a row's item schema may name a
/// [`ValueSchema::Ref`] only the registry can answer, and the registry is a
/// resource this system holds rather than something a closure can own.
#[derive(Component)]
pub(in crate::widgets) struct ValueRebuild {
	/// The children the value asks for, in order, each under the key it is
	/// reconciled by.
	keys: Arc<dyn Fn(&Value) -> Vec<RebuildKey> + Send + Sync>,
	/// Builds the one child `key` names, from the value that asked for it.
	build: Arc<
		dyn for<'a> Fn(SchemaResolver<'a>, &Value, &RebuildKey) -> Snippet
			+ Send
			+ Sync,
	>,
}

/// What a child of a generation is reconciled by: the identity a row, entry or
/// dependent field keeps across changes to the value around it. Never an entity
/// id, which is the reused output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component)]
pub(in crate::widgets) enum RebuildKey {
	/// The one child a collection shows in place of its rows when it has none.
	Empty,
	/// A list row, by position. An authored list item has no identity of its
	/// own, so an append and a pop reuse every surviving index, and a mid-list
	/// removal shifts the rows after the gap onto their new items rather than
	/// moving them; a list whose items carry stable ids upgrades this key
	/// without touching the mechanism.
	Index(usize),
	/// A map entry by its key, an enum's controls by its variant, a struct's
	/// row by its field.
	Name(SmolStr),
	/// A row whose schema a sibling names, by its field and the schema the
	/// sibling's value substituted: rebuilt when its type changes, reused when
	/// only its value does.
	Bound {
		/// The field's key.
		field: SmolStr,
		/// The substituted schema, as a fingerprint.
		schema: SmolStr,
	},
}

impl ValueRebuild {
	/// Hold the `build` of each child `keys` asks for.
	pub(in crate::widgets) fn new(
		keys: impl 'static + Send + Sync + Fn(&Value) -> Vec<RebuildKey>,
		build: impl 'static
		+ Send
		+ Sync
		+ for<'a> Fn(SchemaResolver<'a>, &Value, &RebuildKey) -> Snippet,
	) -> Self {
		Self {
			keys: Arc::new(keys),
			build: Arc::new(build),
		}
	}
}

/// Reconcile the generation of every [`ValueRebuild`] whose bound value
/// changed: reuse each child whose key survived, build each key that is new,
/// despawn each child whose key vanished, and order them as the value asks.
pub(in crate::widgets) fn rebuild_value_widgets(
	schemas: Option<Res<SchemaRegistry>>,
	holders: Populated<
		(Entity, &ValueRebuild, &Value, Option<&Children>),
		Changed<Value>,
	>,
	keys: Query<&RebuildKey>,
	mut commands: Commands,
) {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	for (entity, rebuild, value, children) in holders.iter() {
		let next = (rebuild.keys)(value);
		let children = children
			.into_iter()
			.flat_map(|children| children.iter())
			.collect::<Vec<_>>();
		let mut current = children
			.iter()
			.filter_map(|child| {
				keys.get(*child).ok().map(|key| (key.clone(), *child))
			})
			.collect::<HashMap<_, _>>();
		// the keys are unchanged in an unchanged order: a leaf edit inside
		// the generation, which is the generation's own business
		if children.len() == next.len()
			&& children
				.iter()
				.zip(next.iter())
				.all(|(child, key)| current.get(key) == Some(child))
		{
			continue;
		}
		let generation = next
			.into_iter()
			.map(|key| match current.remove(&key) {
				Some(child) => child,
				None => commands
					.spawn((
						ChildOf(entity),
						(rebuild.build)(resolver, value, &key),
						key,
					))
					.id(),
			})
			.collect::<Vec<_>>();
		// a vanished key, and any child the generation never keyed
		for child in children.iter().filter(|child| !generation.contains(child))
		{
			commands.entity(*child).despawn();
		}
		commands.entity(entity).replace_children(&generation);
	}
}

#[cfg(test)]
mod test {
	use super::RebuildKey;
	use super::ValueRebuild;
	use crate::prelude::*;
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	/// A holder keyed on a list's items: one `<span>` per item, keyed by index,
	/// so an appended item builds one span and an edit *within* an item none.
	fn build() -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<div {(
						FieldRef::new("items"),
						ValueRebuild::new(
							|value| (0..value.as_list().map(Vec::len).unwrap_or_default())
								.map(RebuildKey::Index)
								.collect(),
							|_resolver, value, key| {
								let text = match key {
									RebuildKey::Index(index) => value
										.as_list()
										.ok()
										.and_then(|items| items.get(*index))
										.and_then(|item| item.as_str().ok())
										.unwrap_or_default()
										.to_string(),
									_ => String::new(),
								};
								rsx!{ <span>{text}</span> }.any_snippet()
							},
						),
					)}/>
				</div>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(value!({ "items": ["a"] })));
		world.update_local();
		world.update_local();
		(world, root)
	}

	/// The generation's spans in child order.
	fn spans(world: &mut World) -> Vec<Entity> {
		test_ext::elements_in(world, "span")
	}

	/// Replace the whole list and settle.
	fn set_items(world: &mut World, root: Entity, items: Value) {
		world.entity_mut(root).get_mut::<Document>().unwrap().0 =
			value!({ "items": items });
		world.update_local();
		world.update_local();
	}

	#[beet_core::test]
	fn generates_from_the_bound_value() {
		let (mut world, root) = build();
		test_ext::render_world(&mut world, root)
			.xpect_contains("<span>a</span>");
	}

	/// An append builds only the new key: the existing child keeps its entity,
	/// which is what lets a control keep its focus while a row lands beside it.
	#[beet_core::test]
	fn an_append_reuses_the_surviving_children() {
		let (mut world, root) = build();
		let first = spans(&mut world);
		set_items(&mut world, root, value!(["a", "b"]));
		let second = spans(&mut world);
		second.len().xpect_eq(2);
		second[0].xpect_eq(first[0]);
		test_ext::render_world(&mut world, root)
			.xpect_contains("<span>a</span><span>b</span>");
	}

	/// A vanished key despawns its child and nothing else, so a pop never
	/// leaves the previous generation behind or rebuilds the rows before it.
	#[beet_core::test]
	fn a_vanished_key_despawns_its_child() {
		let (mut world, root) = build();
		set_items(&mut world, root, value!(["a", "b"]));
		let before = spans(&mut world);
		set_items(&mut world, root, value!(["a"]));
		spans(&mut world).xpect_eq(vec![before[0]]);
		world.entities().contains(before[1]).xpect_false();
	}

	/// A change that leaves the keys alone builds nothing: the entity a leaf
	/// lives on survives, so a control keeps its focus and state.
	#[beet_core::test]
	fn an_edit_within_the_keys_rebuilds_nothing() {
		let (mut world, root) = build();
		let generation = spans(&mut world);
		set_items(&mut world, root, value!(["edited"]));
		spans(&mut world).xpect_eq(generation);
	}

	/// A generation follows the order its keys are asked in: a child that
	/// moved is moved, not rebuilt.
	#[beet_core::test]
	fn a_generation_keeps_the_asked_order() {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<div {(
						FieldRef::new("names"),
						ValueRebuild::new(
							|value| value
								.as_list()
								.map(|items| items.iter().filter_map(|item| item.as_str().ok()).map(|item| RebuildKey::Name(item.into())).collect())
								.unwrap_or_default(),
							|_resolver, _value, key| {
								let text = match key {
									RebuildKey::Name(name) => name.to_string(),
									_ => String::new(),
								};
								rsx!{ <span>{text}</span> }.any_snippet()
							},
						),
					)}/>
				</div>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(value!({ "names": ["a", "b"] })));
		world.update_local();
		world.update_local();
		let before = spans(&mut world);
		world.entity_mut(root).get_mut::<Document>().unwrap().0 =
			value!({ "names": ["b", "c", "a"] });
		world.update_local();
		world.update_local();
		let after = spans(&mut world);
		after.len().xpect_eq(3);
		after[0].xpect_eq(before[1]);
		after[2].xpect_eq(before[0]);
		test_ext::render_world(&mut world, root)
			.xpect_contains("<span>b</span><span>c</span><span>a</span>");
	}
}

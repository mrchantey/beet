//! [`ValueRebuild`]: regenerating a value-generated subtree when the shape of
//! the value it was generated from changes.
//!
//! [`SchemaRebuild`](super::SchemaRebuild)'s twin, and the third grain of a
//! schema-driven widget's reactivity. A leaf's *value* rides its own binding and
//! a subtree's *schema* rides the registry; what neither covers is a control
//! whose very shape is decided by the value it edits: a list's rows, a map's
//! entries, an enum's payload, and a field whose schema a sibling names
//! ([`SchemaRef::AtField`]).
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Holds the builder of a subtree the bound [`Value`]'s shape decides, and the
/// shape the current generation was built from.
///
/// Rides a co-located [`FieldRef`] exactly as [`ReactiveChildren`] does: the ref
/// syncs the bound value onto this entity, and a `Changed<Value>` drives the
/// rebuild. The holder must therefore be an **element**, since a `Value` on a
/// tag-less node renders as text; its children are one generation and nothing
/// else, so a rebuild despawns all of them.
///
/// A rebuild fires only when the *shape* changes. Which part of the value is the
/// shape is the builder's to say — a list's length, a map's keys, an enum's
/// variant — so editing a leaf inside the generation never rebuilds the
/// generation that leaf sits in.
///
/// The build closure is handed a live [`SchemaResolver`], because a generation
/// is built long after the walk that authored it: a row's item schema may name a
/// [`ValueSchema::Ref`] only the registry can answer, and the registry is a
/// resource this system holds rather than something a closure can own.
#[derive(Component)]
pub struct ValueRebuild {
	/// What about the value decides the subtree, ie the fingerprint a rebuild is
	/// decided by.
	shape: Arc<dyn Fn(&Value) -> SmolStr + Send + Sync>,
	/// The shape the current generation was built from, `None` until the first
	/// value arrives (which is what spawns the first generation).
	current: Option<SmolStr>,
	/// Builds one generation from the value that shape describes.
	build: Arc<
		dyn for<'a> Fn(SchemaResolver<'a>, &Value) -> Snippet + Send + Sync,
	>,
}

impl ValueRebuild {
	/// Hold the `build` that renders a value, keyed on the `shape` of it.
	pub fn new(
		shape: impl 'static + Send + Sync + Fn(&Value) -> SmolStr,
		build: impl 'static
		+ Send
		+ Sync
		+ for<'a> Fn(SchemaResolver<'a>, &Value) -> Snippet,
	) -> Self {
		Self {
			shape: Arc::new(shape),
			current: None,
			build: Arc::new(build),
		}
	}
}

/// Respawn the generation of every [`ValueRebuild`] whose bound value changed
/// shape, leaving the rest (and every leaf inside an unchanged generation)
/// alone.
pub(super) fn rebuild_value_widgets(
	schemas: Option<Res<SchemaRegistry>>,
	mut holders: Populated<
		(Entity, &mut ValueRebuild, &Value, Option<&Children>),
		Changed<Value>,
	>,
	mut commands: Commands,
) {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	for (entity, mut rebuild, value, children) in holders.iter_mut() {
		let shape = (rebuild.shape)(value);
		if rebuild.current.as_ref() == Some(&shape) {
			continue;
		}
		rebuild.current = Some(shape);
		if let Some(children) = children {
			for child in children.iter() {
				commands.entity(child).despawn();
			}
		}
		commands.spawn((ChildOf(entity), (rebuild.build)(resolver, value)));
	}
}

#[cfg(test)]
mod test {
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A holder keyed on a list's length: an appended item regenerates the
	/// subtree, and an edit *within* an item does not.
	fn build() -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<div {(
						FieldRef::new("items"),
						ValueRebuild::new(
							|value| value.as_list().map(Vec::len).unwrap_or_default().to_string().into(),
							|_resolver, value| rsx!{
								<span>{format!("{} items", value.as_list().map(Vec::len).unwrap_or_default())}</span>
							}.any_snippet(),
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

	#[beet_core::test]
	fn generates_from_the_bound_value() {
		let (mut world, root) = build();
		test_ext::render_world(&mut world, root).xpect_contains("1 items");
	}

	/// The generation is replaced rather than appended to, so a shape change
	/// never leaves the previous one behind.
	#[beet_core::test]
	fn a_shape_change_replaces_the_generation() {
		let (mut world, root) = build();
		world
			.entity_mut(root)
			.get_mut::<Document>()
			.unwrap()
			.0
			.get_mut("items")
			.unwrap()
			.push("b")
			.unwrap();
		world.update_local();
		world.update_local();
		let html = test_ext::render_world(&mut world, root);
		html.clone().xpect_contains("2 items");
		html.xnot().xpect_contains("1 items");
	}

	/// An edit that leaves the shape alone leaves the generation alone: the
	/// entity a leaf lives on survives, so a control keeps its focus and state.
	#[beet_core::test]
	fn an_edit_within_the_shape_rebuilds_nothing() {
		let (mut world, root) = build();
		let generation = world
			.query_once::<(Entity, &Element)>()
			.into_iter()
			.find(|(_, element)| element.tag() == "span")
			.map(|(entity, _)| entity)
			.unwrap();
		world.entity_mut(root).get_mut::<Document>().unwrap().0 =
			value!({ "items": ["edited"] });
		world.update_local();
		world.update_local();
		world.entities().contains(generation).xpect_true();
	}
}

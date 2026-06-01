//! Per-item field-path scoping for [`FieldRef`].
//!
//! Adds a third axis to document resolution, orthogonal to document identity
//! ([`FieldOf`]) and the authored path ([`FieldRef::field_path`]): a field-path
//! *prefix* accumulated from the entity hierarchy.
//!
//! A [`DocumentScope`] on an ancestor contributes path segments to every
//! descendant [`FieldRef`]. The accumulated prefix is materialized as
//! [`ResolvedFieldPath`] (the component the sync systems read) by the
//! [`resolve_field_path`] observer on insert and the
//! [`update_resolved_field_paths`] system on ancestor scope changes.
//!
//! The canonical use is [`ReactiveChildren`]: each spawned child carries its
//! fully-resolved absolute item path as a terminating scope, so a
//! `FieldRef("name")` authored inside child N resolves to `items[N].name`.

use crate::prelude::*;
use beet_core::prelude::*;

/// A field-path prefix contributed to every [`FieldRef`] *beneath* this entity.
///
/// Document-agnostic: contributes only path segments, never decides which
/// document a [`FieldRef`] resolves against (that stays [`FieldRef::document`] /
/// [`FieldOf`]).
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct DocumentScope {
	/// Segments prepended to descendant field paths.
	pub path: FieldPath,
	/// When true, scope collection stops here (this scope is still collected).
	pub terminate: bool,
}

/// Derived from [`FieldRef::field_path`] plus the accumulated ancestor
/// [`DocumentScope`] prefix.
///
/// Recomputed reactively; the authored [`FieldRef`] is never mutated. Holds
/// **only a path** — document identity stays in [`FieldOf`] and `on_missing`
/// stays in [`FieldRef`]. The sync systems consume this component instead of
/// the authored `field_path`.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct ResolvedFieldPath {
	/// scope prefix (outermost first) ++ authored field_path.
	pub field_path: FieldPath,
}

/// Shared upward resolver for the [`DocumentScope`] prefix.
///
/// A slim [`SystemParam`] reused by both [`update_resolved_field_paths`] and
/// [`DocumentQuery`] (which holds it as a field), so read and write resolution
/// share one walk. Splitting it out avoids borrowing all of [`DocumentQuery`]
/// (with its [`Commands`]) in the resolve system.
#[derive(SystemParam)]
pub struct ScopeQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	scopes: Query<'w, 's, &'static DocumentScope>,
}

impl ScopeQuery<'_, '_> {
	/// Accumulate the [`DocumentScope`] prefix for `subject`, walking ancestors
	/// inclusive. Outermost scope leads; a `terminate` scope is collected, then
	/// the walk stops.
	pub fn scope_prefix(&self, subject: Entity) -> FieldPath {
		// innermost -> outermost as we walk up
		let mut collected: Vec<FieldPath> = Vec::new();
		for ancestor in self.ancestors.iter_ancestors_inclusive(subject) {
			if let Ok(scope) = self.scopes.get(ancestor) {
				collected.push(scope.path.clone());
				// a terminate scope is collected, then seals off outer scopes
				if scope.terminate {
					break;
				}
			}
		}
		collected
			.into_iter()
			.rev() // outermost -> innermost
			.flat_map(FieldPath::into_inner)
			.collect::<Vec<_>>()
			.into()
	}

	/// `scope_prefix(subject)` ++ `authored`.
	pub fn resolved_path(
		&self,
		subject: Entity,
		authored: &FieldPath,
	) -> FieldPath {
		let mut path = self.scope_prefix(subject);
		path.extend(authored.iter().cloned());
		path
	}
}

/// Observer that computes [`ResolvedFieldPath`] when a [`FieldRef`] is inserted.
///
/// Mirrors [`link_field_to_document`](super::link_field_to_document), which
/// computes [`FieldOf`] on the same insert. Bundle inserts apply all components
/// before observers fire, so a child's own [`DocumentScope`] and [`ChildOf`] are
/// present when this runs. Handles every newly-inserted ref, including each
/// [`ReactiveChildren`] generation.
pub(super) fn resolve_field_path(
	ev: On<Insert, FieldRef>,
	mut commands: Commands,
	fields: Query<&FieldRef>,
	resolver: ScopeQuery,
) -> Result {
	let field = fields.get(ev.entity)?;
	let field_path = resolver.resolved_path(ev.entity, &field.field_path);
	commands
		.entity(ev.entity)
		.insert(ResolvedFieldPath { field_path });
	Ok(())
}

/// Run condition gating [`update_resolved_field_paths`] to frames where an
/// ancestor scope was added, mutated, removed, or an entity reparented, so the
/// subtree descent does not run on quiet frames.
pub(super) fn resolved_paths_need_update(
	changed_scopes: Query<(), Changed<DocumentScope>>,
	reparented: Query<(), Changed<ChildOf>>,
	removed_scopes: RemovedComponents<DocumentScope>,
) -> bool {
	!changed_scopes.is_empty()
		|| !reparented.is_empty()
		|| !removed_scopes.is_empty()
}

/// System that recomputes [`ResolvedFieldPath`] for existing refs whose
/// *ancestor* scope changed — a change the descendant's own change ticks cannot
/// see.
///
/// For each changed entity (scope add/mutate, scope removal, or reparent),
/// descend its subtree and recompute every [`FieldRef`] found, pruning subtrees
/// that a terminating scope seals off. The descent only *selects* refs; the path
/// is always produced by the shared upward resolver, and the equality guard
/// absorbs redundant recomputes.
pub(super) fn update_resolved_field_paths(
	changed_scopes: Query<Entity, Changed<DocumentScope>>,
	reparented: Query<Entity, Changed<ChildOf>>,
	mut removed_scopes: RemovedComponents<DocumentScope>,
	children: Query<&Children>,
	scopes: Query<&DocumentScope>,
	resolver: ScopeQuery,
	has_ref: Query<&FieldRef>,
	mut resolved: Query<&mut ResolvedFieldPath>,
) -> Result {
	let roots = changed_scopes
		.iter()
		.chain(reparented.iter())
		.chain(removed_scopes.read());
	for root in roots {
		let mut stack = vec![root];
		while let Some(node) = stack.pop() {
			if let (Ok(field), Ok(mut slot)) =
				(has_ref.get(node), resolved.get_mut(node))
			{
				let next = resolver.resolved_path(node, &field.field_path);
				// equality guard: only dirty when the path actually changed
				if slot.field_path != next {
					slot.field_path = next;
				}
			}
			if let Ok(kids) = children.get(node) {
				kids.iter()
					// a terminating child scope seals itself + its subtree from `root`
					.filter(|child| {
						scopes.get(*child).map_or(true, |scope| !scope.terminate)
					})
					.for_each(|child| stack.push(child));
			}
		}
	}
	Ok(())
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	/// Read the local [`Value`] of `entity`.
	fn read_value(world: &mut World, entity: Entity) -> Value {
		world.entity(entity).get::<Value>().unwrap().clone()
	}

	/// Read the [`ResolvedFieldPath`] of `entity`.
	fn resolved_path(world: &mut World, entity: Entity) -> FieldPath {
		world
			.entity(entity)
			.get::<ResolvedFieldPath>()
			.unwrap()
			.field_path
			.clone()
	}

	#[beet_core::test]
	fn scope_prepends_prefix() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "user": { "name": "Alice" } })))
			.id();
		// a DocumentScope ancestor scoping descendants into "user"
		let scope = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["user"]),
				terminate: false,
			}))
			.id();
		let field = world
			.spawn((ChildOf(scope), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		resolved_path(&mut world, field)
			.xpect_eq(FieldPath::new(["user", "name"]));
		read_value(&mut world, field).xpect_eq(Value::Str("Alice".into()));
	}

	#[beet_core::test]
	fn scopes_compose() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(
				val!({ "user": { "address": { "city": "NYC" } } }),
			))
			.id();
		let outer = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["user"]),
				terminate: false,
			}))
			.id();
		let inner = world
			.spawn((ChildOf(outer), DocumentScope {
				path: FieldPath::new(["address"]),
				terminate: false,
			}))
			.id();
		let field = world
			.spawn((ChildOf(inner), Value::default(), FieldRef::new("city")))
			.id();
		world.update_local();

		resolved_path(&mut world, field)
			.xpect_eq(FieldPath::new(["user", "address", "city"]));
		read_value(&mut world, field).xpect_eq(Value::Str("NYC".into()));
	}

	#[beet_core::test]
	fn terminate_seals_outer_scope() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "name": "top" })))
			.id();
		// an outer scope that would otherwise prefix "outer"
		let outer = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["outer"]),
				terminate: false,
			}))
			.id();
		// a terminating scope with an empty prefix seals off "outer"
		let sealed = world
			.spawn((ChildOf(outer), DocumentScope {
				path: FieldPath::default(),
				terminate: true,
			}))
			.id();
		let field = world
			.spawn((ChildOf(sealed), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		// "outer" is sealed off, so the path is just ["name"]
		resolved_path(&mut world, field).xpect_eq(FieldPath::new(["name"]));
		read_value(&mut world, field).xpect_eq(Value::Str("top".into()));
	}

	#[beet_core::test]
	fn scope_change_recomputes() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({
				"a": { "name": "from_a" },
				"b": { "name": "from_b" }
			})))
			.id();
		let scope = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["a"]),
				terminate: false,
			}))
			.id();
		let field = world
			.spawn((ChildOf(scope), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		read_value(&mut world, field).xpect_eq(Value::Str("from_a".into()));

		// mutate the ancestor scope, no document change
		world.entity_mut(scope).get_mut::<DocumentScope>().unwrap().path =
			FieldPath::new(["b"]);
		world.update_local();

		// the descendant re-syncs to the new scoped path
		resolved_path(&mut world, field).xpect_eq(FieldPath::new(["b", "name"]));
		read_value(&mut world, field).xpect_eq(Value::Str("from_b".into()));
	}

	#[beet_core::test]
	fn scope_removed_recomputes() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({
				"name": "root_name",
				"a": { "name": "from_a" }
			})))
			.id();
		let scope = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["a"]),
				terminate: false,
			}))
			.id();
		let field = world
			.spawn((ChildOf(scope), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		read_value(&mut world, field).xpect_eq(Value::Str("from_a".into()));

		// remove the scope: the ref resolves to the bare path
		world.entity_mut(scope).remove::<DocumentScope>();
		world.update_local();

		resolved_path(&mut world, field).xpect_eq(FieldPath::new(["name"]));
		read_value(&mut world, field).xpect_eq(Value::Str("root_name".into()));
	}

	#[beet_core::test]
	fn reparent_recomputes() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({
				"a": { "name": "from_a" },
				"b": { "name": "from_b" }
			})))
			.id();
		let scope_a = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["a"]),
				terminate: false,
			}))
			.id();
		let scope_b = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["b"]),
				terminate: false,
			}))
			.id();
		let field = world
			.spawn((ChildOf(scope_a), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		read_value(&mut world, field).xpect_eq(Value::Str("from_a".into()));

		// reparent under the other scope
		world.entity_mut(field).insert(ChildOf(scope_b));
		world.update_local();

		resolved_path(&mut world, field).xpect_eq(FieldPath::new(["b", "name"]));
		read_value(&mut world, field).xpect_eq(Value::Str("from_b".into()));
	}

	#[beet_core::test]
	fn write_path_is_scoped() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "user": { "name": "old" } })))
			.id();
		let scope = world
			.spawn((ChildOf(doc), DocumentScope {
				path: FieldPath::new(["user"]),
				terminate: false,
			}))
			.id();
		let field = world
			.spawn((ChildOf(scope), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		// a write from the scoped subject lands at user.name, not name
		let field_ref = FieldRef::new("name");
		world
			.run_system_cached_with(
				|In((subject, field_ref)): In<(Entity, FieldRef)>,
				 mut docs: DocumentQuery| {
					docs.with_field(subject, &field_ref, |value| {
						*value = Value::Str("new".into())
					})
					.unwrap();
				},
				(field, field_ref),
			)
			.unwrap();

		let document = world.entity(doc).get::<Document>().unwrap().0.clone();
		document.xpect_eq(val!({ "user": { "name": "new" } }));
	}
}

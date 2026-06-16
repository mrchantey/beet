//! Node ancestry over the combined element + attribute tree.
//!
//! An [`Element`]'s [`Attribute`] nodes are separate entities linked by
//! [`AttributeOf`], outside the [`ChildOf`] tree. A plain `ChildOf` walk from an
//! attribute entity therefore finds neither its owning element nor the
//! surrounding document. [`ElementTraverseQuery`] is the single ancestry walk
//! that bridges the two: every step takes the `ChildOf` parent, falling back to
//! the `AttributeOf` owner, so all node ancestry funnels through one place.

use crate::prelude::*;
use bevy::ecs::relationship::Relationship;

/// Read-only ancestry walk over the element + attribute tree.
///
/// Each step prioritises [`ChildOf`], hopping [`AttributeOf`] only when no
/// `ChildOf` parent exists, so an attribute entity walks up through its owning
/// element. The shared single step is
/// [`element_parent`](Self::element_parent), and
/// [`world_ancestors_inclusive`](Self::world_ancestors_inclusive) runs the same
/// walk directly over a `&World` for callers outside a system (see
/// [`BindingTarget::resolve`](crate::prelude::BindingTarget::resolve)).
#[derive(SystemParam)]
pub struct ElementTraverseQuery<'w, 's> {
	child_of: Query<'w, 's, &'static ChildOf>,
	attribute_of: Query<'w, 's, &'static AttributeOf>,
}

impl ElementTraverseQuery<'_, '_> {
	/// The single ancestry step: the `ChildOf` parent, else the `AttributeOf`
	/// owner, `None` at the root.
	pub fn parent(&self, entity: Entity) -> Option<Entity> {
		ElementTraverseQuery::element_parent(
			self.child_of.get(entity).ok(),
			self.attribute_of.get(entity).ok(),
		)
	}

	/// The topmost ancestor reached by repeated [`parent`](Self::parent) steps,
	/// or `entity` itself when it has no parent.
	pub fn root(&self, entity: Entity) -> Entity {
		self.ancestors_inclusive(entity).last().unwrap_or(entity)
	}

	/// Iterate `entity` then each ancestor (inclusive), every step taking the
	/// `ChildOf` parent then falling back to the `AttributeOf` owner. Loop-safe:
	/// stops on the first revisited entity, so a malformed self- or
	/// `AttributeOf`-referential graph cannot spin forever.
	pub fn ancestors_inclusive(
		&self,
		entity: Entity,
	) -> impl Iterator<Item = Entity> + '_ {
		let mut visited = HashSet::<Entity>::default();
		visited.insert(entity);
		// `successors` yields `entity`, then each `parent`, stopping the first
		// time a parent has already been visited.
		core::iter::successors(Some(entity), move |&current| {
			self.parent(current).filter(|parent| visited.insert(*parent))
		})
	}
}

impl ElementTraverseQuery<'static, 'static> {
	/// The single element-ancestry step, shared by [`ElementTraverseQuery`]'s
	/// query walk and the `&World` walk below: the `ChildOf` parent, else the
	/// `AttributeOf` owner (an attribute entity lives outside the `ChildOf` tree,
	/// so it hops to its owning element). `ChildOf` is prioritised.
	pub fn element_parent(
		child_of: Option<&ChildOf>,
		attribute_of: Option<&AttributeOf>,
	) -> Option<Entity> {
		child_of
			.map(|child_of| child_of.parent())
			.or_else(|| attribute_of.map(|attribute_of| attribute_of.get()))
	}

	/// `entity` then each element-ancestor, walked directly over a `&World` for
	/// callers outside a system (eg
	/// [`BindingTarget::resolve`](crate::prelude::BindingTarget::resolve)).
	/// Mirrors [`ancestors_inclusive`](Self::ancestors_inclusive), which needs the
	/// query's borrowed access, and is loop-safe the same way (stops on the first
	/// revisited entity).
	pub fn world_ancestors_inclusive(
		world: &World,
		entity: Entity,
	) -> impl Iterator<Item = Entity> + '_ {
		let mut visited = HashSet::<Entity>::default();
		visited.insert(entity);
		core::iter::successors(Some(entity), move |&current| {
			world
				.get_entity(current)
				.ok()
				.and_then(|entity_ref| {
					Self::element_parent(
						entity_ref.get::<ChildOf>(),
						entity_ref.get::<AttributeOf>(),
					)
				})
				.filter(|parent| visited.insert(*parent))
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// An attribute entity sits outside the `ChildOf` tree: the walk must hop
	/// `AttributeOf` to reach the owning element and continue to the root.
	#[beet_core::test]
	fn walks_child_of_then_attribute_of() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		let element = world.spawn(ChildOf(root)).id();
		// an attribute entity related to the element, not a ChildOf child
		let attribute = world.spawn(AttributeOf::new(element)).id();

		world
			.run_system_cached_with(
				|input: In<(Entity, Entity, Entity)>,
				 traverse: ElementTraverseQuery| {
					let (root, element, attribute) = *input;
					// the attribute hops to its element, then to the root
					traverse
						.ancestors_inclusive(attribute)
						.collect::<Vec<_>>()
						.xpect_eq(vec![attribute, element, root]);
					traverse.root(attribute).xpect_eq(root);
					traverse.parent(attribute).xpect_eq(Some(element));
					// a plain ChildOf child walks the tree as usual
					traverse.parent(element).xpect_eq(Some(root));
					traverse.parent(root).xpect_eq(None);
				},
				(root, element, attribute),
			)
			.unwrap();
	}
}

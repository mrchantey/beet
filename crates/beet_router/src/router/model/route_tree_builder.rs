//! The one `SystemParam` a [`RouteTree`] rebuild threads, instead of four
//! queries and [`Commands`] passed separately.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Everything a [`RouteTree`] rebuild needs, bundled into one system param so
/// a caller threads one value instead of four queries and [`Commands`]
/// separately.
#[derive(SystemParam)]
pub struct RouteTreeBuilder<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	paths: Query<'w, 's, &'static PathPartial>,
	actions: Query<'w, 's, ActionQueryItem<'static>, Without<RouteHidden>>,
	existing_trees: Query<'w, 's, Entity, With<RouteTree>>,
	commands: Commands<'w, 's>,
}

impl RouteTreeBuilder<'_, '_> {
	/// The namespace `entity` belongs to: the nearest ancestor `Router`
	/// ([`PathPattern::namespace_root`]), else the document root.
	pub fn namespace_of(&self, entity: Entity) -> Entity {
		PathPattern::namespace_root(entity, &self.ancestors, &self.paths)
	}

	/// Every entity that currently carries a [`RouteTree`].
	pub fn existing_roots(&self) -> impl Iterator<Item = Entity> + '_ {
		self.existing_trees.iter()
	}

	/// The one grouping walk every [`RouteTree`] rebuild trigger runs: descend
	/// `root`'s subtree, bucket every live route by its own
	/// [`PathPattern::namespace_root`] (a nested `Router` owns its own
	/// namespace and tree, so its routes never bucket to an ancestor), and
	/// insert a fresh tree per bucket.
	///
	/// Every namespace, not just `root`'s: a mounted scene rooted in its own
	/// `Router` is a url space of its own, and reparenting is exactly when its
	/// ancestry (and so its namespace) settles.
	///
	/// Any subtree entity that currently carries a [`RouteTree`] but is not a
	/// bucket this pass — no longer a namespace root (reparented away), or a
	/// namespace root left with no live routes — has it removed. This closes
	/// the phantom-tree class: a stale tree that would otherwise keep
	/// dispatching routes that no longer live there.
	pub fn rebuild_subtree(&mut self, root: Entity) -> Result {
		let mut spaces: Vec<(Entity, Vec<ActionNode>)> = Vec::new();
		for item in self.actions.iter() {
			if !self.is_at_or_under(item.0, root) {
				continue;
			}
			let space_root = self.namespace_of(item.0);
			let node = ActionNode::from_query(item);
			match spaces.iter_mut().find(|(space, _)| *space == space_root) {
				Some((_, nodes)) => nodes.push(node),
				None => spaces.push((space_root, vec![node])),
			}
		}
		let stale: Vec<Entity> = self
			.existing_trees
			.iter()
			.filter(|entity| self.is_at_or_under(*entity, root))
			.filter(|entity| !spaces.iter().any(|(space, _)| space == entity))
			.collect();
		for entity in stale {
			self.commands.entity(entity).remove::<RouteTree>();
		}
		for (space_root, nodes) in spaces {
			self.commands
				.entity(space_root)
				.insert(RouteTree::from_nodes(nodes)?);
		}
		Ok(())
	}

	/// Whether `entity` is `root` or sits anywhere beneath it.
	///
	/// Walks up from `entity` through [`ChildOf`] rather than down from `root`
	/// through [`Children`]: a child's own `ChildOf` is set the moment it is
	/// spawned, while the parent's `Children` is maintained by a deferred
	/// relationship hook, so descending sees a hierarchy one command flush
	/// stale — which silently drops the routes most recently added to a router.
	fn is_at_or_under(&self, entity: Entity, root: Entity) -> bool {
		self.ancestors
			.iter_ancestors_inclusive_once::<ChildOf>(entity)
			.any(|ancestor| ancestor == root)
	}
}

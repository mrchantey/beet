//! The one `SystemParam` a [`RouteTree`] rebuild threads, instead of its
//! queries and [`Commands`] passed separately.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Everything a [`RouteTree`] rebuild needs, bundled into one system param so
/// a caller threads one value instead of its queries and [`Commands`]
/// separately.
#[derive(SystemParam)]
pub struct RouteTreeBuilder<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	paths: Query<'w, 's, &'static PathPartial>,
	// the url space roots, bucketed ahead of the routes that land under them
	space_roots: Query<'w, 's, (Entity, &'static PathPartial)>,
	actions: Query<'w, 's, ActionQueryItem<'static>, Without<RouteHidden>>,
	params: ParamsQuery<'w, 's>,
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
	/// Every url space root in the subtree is a bucket from the moment it
	/// exists, empty until its routes land: a `Router` declared ahead of its
	/// routes (a scaffolded entry, a router filled at runtime) answers
	/// not-found, rather than failing every dispatch, `--help` included, for
	/// want of a tree.
	///
	/// Every namespace, not just `root`'s: a mounted scene rooted in its own
	/// `Router` is a url space of its own, and reparenting is exactly when its
	/// ancestry (and so its namespace) settles.
	///
	/// Any subtree entity that currently carries a [`RouteTree`] but is not a
	/// bucket this pass, no longer a namespace root (reparented away) or a
	/// document-root fallback left with no live routes, has it removed. This
	/// closes the phantom-tree class: a stale tree that would otherwise keep
	/// dispatching routes that no longer live there.
	pub fn rebuild_subtree(&mut self, root: Entity) -> Result {
		// the url space roots first, so a routeless one still gets its tree
		let mut spaces: Vec<(Entity, Vec<ActionNode>)> = self
			.space_roots
			.iter()
			.filter(|(entity, partial)| {
				partial.is_root && self.is_at_or_under(*entity, root)
			})
			.map(|(entity, _)| (entity, Vec::new()))
			.collect();
		for item in self.actions.iter() {
			if !self.is_at_or_under(item.0, root) {
				continue;
			}
			let space_root = self.namespace_of(item.0);
			let node = ActionNode::from_query(item, self.params.of(item.0)?);
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Two readers on one route naming one key with different types are a
	/// contradiction, refused by the tree build rather than at dispatch.
	#[beet_core::test]
	fn a_param_conflict_fails_the_build() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct PortNumber {
			port: u16,
		}
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct PortName {
			port: String,
		}
		let mut world = World::new();
		let root = world.spawn(SubtreeParams::new::<PortNumber>()).id();
		world.spawn((
			ChildOf(root),
			ActionMeta::of::<(), Request, Response>(),
			PathPattern::new("serve").unwrap(),
			RouteParams::new::<PortName>(),
		));
		world
			.run_system_once_with::<_, _, Result, _>(RouteTree::rebuild, root)
			.unwrap()
			.unwrap_err()
			.to_string()
			.xpect_contains("conflicting declarations of `--port`");
	}

	/// A router owns its url space from the moment it exists: with no routes
	/// its tree is empty, so a request into it answers not-found rather than
	/// failing dispatch for want of a tree.
	#[beet_core::test]
	async fn routeless_router_owns_an_empty_tree() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let router = world.spawn(Router).flush();
		RouteTree::of(&world, router)
			.unwrap()
			.find(&["anything"])
			.xpect_none();
		world
			.entity_mut(router)
			.exchange(Request::get("anything"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}
}

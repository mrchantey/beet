mod help;
mod router;
pub use help::*;
pub use router::*;
mod navigate;
pub use navigate::*;
mod render_markdown;
pub use render_markdown::*;
mod render_tui;
pub use render_tui::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// Gets the [`RouteTree`] from the root ancestor of the given entity.
pub(crate) fn root_route_tree(
	world: &World,
	entity: Entity,
) -> Result<&RouteTree> {
	/// Walks up [`ChildOf`] relations to find the root ancestor entity.
	fn walk_to_root(world: &World, entity: Entity) -> Entity {
		let mut current = entity;
		while let Some(child_of) = world.entity(current).get::<ChildOf>() {
			current = child_of.parent();
		}
		current
	}

	let root = walk_to_root(world, entity);
	world
		.entity(root)
		.get::<RouteTree>()
		.ok_or_else(|| bevyhow!("No RouteTree found on root ancestor"))
}

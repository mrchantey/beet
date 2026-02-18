use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers route-building observers for tools.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor
/// whenever tools are spawned in an entity hierarchy. Cards now
/// register as tools (via [`Card`] + [`ToolMeta`]), so there
/// is no separate card observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(insert_tool_path_and_params)
			.add_observer(insert_route_tree);
	}
}

/// Observer that listens for new tools and inserts their path and params patterns.
/// Any [`PathPartial`] or [`ParamsPartial`] will be collected so long as they are
/// spawned at the same time as the tool, even if they come after it in the tuple.
/// This is because, unlike OnAdd component hooks, observers run after the entire
/// tree is spawned.
pub fn insert_tool_path_and_params(
	ev: On<Insert, ToolMeta>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	mut commands: Commands,
) -> Result {
	insert_path_and_params(
		ev.entity,
		&ancestors,
		&paths,
		&params,
		&mut commands,
	)
}

/// Shared logic for collecting ancestor path and param partials
/// and inserting the resolved [`PathPattern`] and [`ParamsPattern`]
/// on the given entity.
fn insert_path_and_params(
	entity: Entity,
	ancestors: &Query<&ChildOf>,
	paths: &Query<&PathPartial>,
	params: &Query<&ParamsPartial>,
	commands: &mut Commands,
) -> Result {
	let path = PathPattern::collect(entity, ancestors, paths)?;
	let params = ParamsPattern::collect(entity, ancestors, params)?;
	commands.entity(entity).insert((path, params));
	Ok(())
}

/// Observer that rebuilds the [`RouteTree`] on the root ancestor
/// whenever a [`PathPattern`] is inserted on any entity in the hierarchy.
///
/// Collects all entities with tool components ([`ToolMeta`], [`PathPattern`],
/// [`ParamsPattern`]) from the root's descendants and constructs a validated
/// tree. Cards are distinguished from regular tools by the presence of a
/// [`Card`] component, which sets `is_card: true` on the [`ToolNode`].
pub fn insert_route_tree(
	ev: On<Insert, PathPattern>,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	tools: Query<
		(
			Entity,
			&ToolMeta,
			&PathPattern,
			&ParamsPattern,
			Option<&HttpMethod>,
			Option<&Card>,
		),
		Without<RouteHidden>,
	>,
	mut commands: Commands,
) -> Result {
	let root = ancestors.root_ancestor(ev.entity);
	let mut nodes: Vec<ToolNode> = Vec::new();

	for entity in children_query.iter_descendants_inclusive(root) {
		if let Ok(item) = tools.get(entity) {
			nodes.push(ToolNode::from_query(item));
		}
	}

	if nodes.is_empty() {
		return Ok(());
	}

	let tree = RouteTree::from_nodes(nodes)?;
	commands.entity(root).insert(tree);

	Ok(())
}

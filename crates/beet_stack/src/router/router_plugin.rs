use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers route-building observers for tools and cards.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor
/// whenever tools or cards are spawned in an entity hierarchy.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(insert_tool_path_and_params)
			.add_observer(insert_card_path_and_params)
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

/// Observer that listens for new cards and inserts their path and params patterns.
/// Shares the same collection logic as [`insert_tool_path_and_params`],
/// ensuring cards are also routable entries in the [`RouteTree`].
pub fn insert_card_path_and_params(
	ev: On<Insert, Card>,
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
/// [`ParamsPattern`]) and card components ([`Card`], [`PathPattern`],
/// [`ParamsPattern`]) from the root's descendants and constructs a validated tree.
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
			Has<ExchangeToolMarker>,
		),
		// With<ExchangeToolMarker>,
	>,
	cards: Query<
		(Entity, &PathPattern, &ParamsPattern),
		(With<Card>, Without<ToolMeta>),
	>,
	mut commands: Commands,
) -> Result {
	let root = ancestors.root_ancestor(ev.entity);
	let mut nodes: Vec<RouteNode> = Vec::new();

	for entity in children_query.iter_descendants_inclusive(root) {
		if let Ok(item) = tools.get(entity) {
			nodes.push(RouteNode::Tool(ToolNode::from_query(item)));
		} else if let Ok(item) = cards.get(entity) {
			nodes.push(RouteNode::Card(CardNode::from_query(item)));
		}
	}

	if nodes.is_empty() {
		return Ok(());
	}

	let tree = RouteTree::from_nodes(nodes)?;
	commands.entity(root).insert(tree);
	Ok(())
}

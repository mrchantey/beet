use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_action::prelude::*;


/// Plugin that registers route-building observers for actions.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor
/// whenever actions are spawned in an entity hierarchy. Scene routes
/// register as actions (via [`DocumentScope`] + [`ActionMeta`]), so there
/// is no separate scene observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<HelpHandler>()
			.register_type::<NavigateHandler>()
			.register_type::<PathPartial>()
			.add_observer(insert_action_path_and_params)
			.add_observer(insert_route_tree);
	}
}

/// Observer that listens for new actions and inserts their path and params patterns.
/// Any [`PathPartial`] or [`ParamsPartial`] will be collected so long as they are
/// spawned at the same time as the action, even if they come after it in the tuple.
/// This is because, unlike OnAdd component hooks, observers run after the entire
/// tree is spawned.
///
/// Only actions whose entity directly carries a [`PathPartial`] become routes.
/// Descendants of a route entity (eg sequence steps) are skipped.
pub fn insert_action_path_and_params(
	ev: On<Insert, ActionMeta>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	mut commands: Commands,
) -> Result {
	// only entities that have their own PathPartial become routes;
	// children of a route (eg sequence steps) have no PathPartial themselves
	if !paths.contains(ev.entity) {
		return Ok(());
	}
	let path = PathPattern::collect(ev.entity, &ancestors, &paths)?;
	let params = ParamsPattern::collect(ev.entity, &ancestors, &params)?;
	commands.entity(ev.entity).insert((path, params));
	Ok(())
}

/// Observer that rebuilds the [`RouteTree`] on the root ancestor
/// whenever a [`PathPattern`] is inserted on any entity in the hierarchy.
///
/// Collects all entities with action components ([`ActionMeta`], [`PathPattern`],
/// [`ParamsPattern`]) from the root's descendants and constructs a validated
/// tree. Scene routes are distinguished from regular actions by their output
/// type being [`SceneEntity`], detected vian [`ActionMeta::output_is`].
// TODO this is a bit wasteful, if we used change detection could deduplicate added,
// and only generate once, but we'd still want a guanratee the system runs immediately
pub fn insert_route_tree(
	ev: On<Insert, PathPattern>,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	actions: Query<ActionQueryItem, Without<RouteHidden>>,
	mut commands: Commands,
) -> Result {
	let root = ancestors.root_ancestor(ev.entity);
	let mut nodes: Vec<ActionNode> = Vec::new();
	// when added via ChildOf, it will not have been added to the Children,
	// so we check this one manually
	if let Ok(item) = actions.get(ev.entity) {
		nodes.push(ActionNode::from_query(item));
	}

	for entity in children_query
		.iter_descendants_inclusive(root)
		// we've already checked this one
		.filter(|entity| *entity != ev.entity)
	{
		if let Ok(item) = actions.get(entity) {
			nodes.push(ActionNode::from_query(item));
		}
	}

	if nodes.is_empty() {
		return Ok(());
	}

	let tree = RouteTree::from_nodes(nodes)?;
	commands.entity(root).insert(tree);

	Ok(())
}

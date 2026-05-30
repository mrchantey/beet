use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use bevy::asset::AssetPlugin;


/// Plugin that registers route-building observers for actions.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor
/// whenever actions are spawned in an entity hierarchy. Scene routes
/// register as actions (via [`RenderRoot`] + [`ActionMeta`]), so there
/// is no separate scene observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ActionPlugin>()
			.init_plugin::<AsyncPlugin>()
			// scene routes render through the charcell layout/paint pipeline
			// (help pages, markdown/html scenes → ANSI/text); without it the
			// `PostParseTree` schedule has no systems and ANSI output is blank.
			.init_plugin::<CharcellPlugin>()
			// `scene_route` spawns Bevy scenes per request; needs the
			// AssetServer and ScenePatch asset machinery.
			.init_plugin::<AssetPlugin>()
			.init_plugin::<ScenePlugin>()
			.register_type::<HelpHandler>()
			.register_type::<NavigateHandler>()
			.register_type::<InterruptOnRun>()
			.register_type::<InterruptOnEnd>()
			.register_type::<PathPartial>()
			.register_type::<ParamsPartial>()
			.register_type::<PathPattern>()
			.register_type::<ParamsPattern>()
			.register_type::<RequestLogger>()
			.register_type::<NoCacheHeaders>()
			.register_type::<CorsHandler>()
			.register_type::<CorsConfig>()
			.register_type::<HtmlStoreAction>()
			.register_type::<Router>()
			.add_observer(insert_action_path_and_params)
			.add_observer(insert_path_pattern_for_late_path_partial)
			.add_observer(insert_route_tree)
			.add_systems(Update, rebuild_route_trees_after_load);
		#[cfg(feature = "scripting")]
		app.register_type::<Script<RequestParts, String>>()
			.register_type::<ExchangeScript<(), String>>()
			.register_type::<
				ExchangeScript<RequestParts, String, RequestParts, SerdeIntoResponseMarker>,
			>();
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

/// Observer that catches the scene-load case where [`PathPartial`] is
/// inserted *after* [`ActionMeta`], so [`insert_action_path_and_params`]
/// would have short-circuited. Re-runs the path/params collection here.
pub fn insert_path_pattern_for_late_path_partial(
	ev: On<Insert, PathPartial>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	actions: Query<(), (With<ActionMeta>, Without<PathPattern>)>,
	mut commands: Commands,
) -> Result {
	// ActionMeta must already be present and PathPattern not yet computed
	if !actions.contains(ev.entity) {
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
/// type being [`RenderRequest`], detected via [`ActionMeta::output_is`].
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

/// System that rebuilds [`RouteTree`] roots after scene loads, where
/// reflect-driven [`ChildOf`] inserts settle later than [`PathPattern`]
/// and leave per-leaf trees on the wrong ancestors.
///
/// Each tick, every root with a freshly-added [`PathPattern`]
/// descendant is recomputed once.
///
/// TODO to be addressed after the new bevy scene system in 0.19,
/// which should provide after-scene-load hooks so this can be
/// replaced with a single one-shot pass.
pub fn rebuild_route_trees_after_load(
	mut commands: Commands,
	added_paths: Populated<Entity, Added<PathPattern>>,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	actions: Query<ActionQueryItem, Without<RouteHidden>>,
) -> Result {
	// collect unique roots so we rebuild each tree at most once per tick
	let mut roots: Vec<Entity> = added_paths
		.iter()
		.map(|entity| ancestors.root_ancestor(entity))
		.collect();
	roots.sort();
	roots.dedup();

	for root in roots {
		let nodes: Vec<ActionNode> = children_query
			.iter_descendants_inclusive(root)
			.filter_map(|entity| actions.get(entity).ok())
			.map(ActionNode::from_query)
			.collect();
		if nodes.is_empty() {
			continue;
		}
		let tree = RouteTree::from_nodes(nodes)?;
		commands.entity(root).insert(tree);
	}
	Ok(())
}

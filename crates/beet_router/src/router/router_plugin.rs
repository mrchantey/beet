use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "std")]
use beet_ui::prelude::*;
#[cfg(feature = "std")]
use bevy::asset::AssetPlugin;


/// Plugin that registers route-building observers for actions.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor whenever
/// actions are spawned in an entity hierarchy. The route-building observers are
/// shared across std and no_std; the std build additionally wires the scene /
/// asset / charcell rendering pipeline and the reflect registrations the
/// help/scene routes and `world_serde`/scripting need (all std-only). Scene
/// routes register as actions (via [`RenderRoot`] + [`ActionMeta`]), so there is
/// no separate scene observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ActionPlugin>()
			.init_plugin::<AsyncPlugin>()
			.add_observer(insert_action_path_and_params)
			.add_observer(insert_path_pattern_for_late_path_partial)
			.add_observer(insert_route_tree);

		// no_std-core reflect registrations: these types are shared across std
		// and no_std and reflection works on bare metal, so register them
		// unconditionally to keep scene-loading reflection available on no_std.
		// `register_type` initialises the `AppTypeRegistry` if the app has not
		// added one, so this is safe without an explicit registry.
		app.register_type::<InterruptOnRun>()
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
			.register_type::<Router>();

		// std-only: the scene/asset/charcell rendering pipeline (help pages,
		// markdown/html scenes → ANSI/text) and the reflect registrations for
		// the help/navigate middleware, which live in the std-only `help` /
		// `navigate` render-media modules. no_std routers dispatch and fall back
		// to plain text without any of this.
		#[cfg(feature = "std")]
		{
			app
				// scene routes render through the charcell layout/paint pipeline;
				// without it the `PostParseTree` schedule has no systems and ANSI
				// output is blank.
				.init_plugin::<CharcellPlugin>()
				// `scene_route` spawns Bevy scenes per request; needs the
				// AssetServer and ScenePatch asset machinery.
				.init_plugin::<AssetPlugin>()
				.init_plugin::<ScenePlugin>()
				.register_type::<HelpHandler>()
				.register_type::<NavigateHandler>();
			#[cfg(feature = "world_serde")]
			app.add_observer(rebuild_route_trees_on_load);
			#[cfg(feature = "scripting")]
			app.register_type::<Script<RequestParts, String>>()
				.register_type::<ExchangeScript<(), String>>()
				.register_type::<ExchangeScript<
					RequestParts,
					String,
					RequestParts,
					SerdeIntoResponseMarker,
				>>();
		}
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

/// Observer that rebuilds [`RouteTree`] roots after a [`WorldSerdeLoaded`],
/// where reflect-driven [`ChildOf`] inserts settle later than [`PathPattern`]
/// and leave per-leaf trees on the wrong ancestors.
///
/// The load trigger fires synchronously once the hierarchy is whole, so each
/// affected root is recomputed exactly once before any async serving begins.
#[cfg(feature = "world_serde")]
pub fn rebuild_route_trees_on_load(
	ev: On<WorldSerdeLoaded>,
	mut commands: Commands,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	actions: Query<ActionQueryItem, Without<RouteHidden>>,
) -> Result {
	// collect unique roots so we rebuild each tree at most once
	let mut roots: Vec<Entity> = ev
		.entities
		.iter()
		.map(|entity| ancestors.root_ancestor(*entity))
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

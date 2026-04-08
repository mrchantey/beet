use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;


/// Plugin that registers route-building observers for tools.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor
/// whenever tools are spawned in an entity hierarchy. Scene routes
/// register as tools (via [`SceneRoute`] + [`ToolMeta`]), so there
/// is no separate scene observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<HelpHandler>()
			.register_type::<ContextualNotFoundHandler>()
			.register_type::<NavigateHandler>()
			.register_type::<RouterTool>()
			.register_type::<PathPartial>()
			.add_observer(insert_tool_path_and_params)
			.add_observer(insert_route_tree);
	}
}

/// Observer that listens for new tools and inserts their path and params patterns.
/// Any [`PathPartial`] or [`ParamsPartial`] will be collected so long as they are
/// spawned at the same time as the tool, even if they come after it in the tuple.
/// This is because, unlike OnAdd component hooks, observers run after the entire
/// tree is spawned.
///
/// Control-flow tools (ie [`Sequence`], [`Repeat`]) that have no [`PathPartial`]
/// anywhere in their hierarchy are skipped — they are not HTTP routes.
pub fn insert_tool_path_and_params(
	ev: On<Insert, ToolMeta>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	mut commands: Commands,
) -> Result {
	// skip tools with no explicit path — control-flow tools like Sequence and
	// RepeatTimes require a Tool via #[require] but should not become routes
	let has_path = ancestors
		.iter_ancestors_inclusive(ev.entity)
		.any(|entity| paths.contains(entity));
	if !has_path {
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
/// Collects all entities with tool components ([`ToolMeta`], [`PathPattern`],
/// [`ParamsPattern`]) from the root's descendants and constructs a validated
/// tree. Scene routes are distinguished from regular tools by the presence of
/// a [`SceneRoute`] component, which sets `is_scene: true` on the [`ToolNode`].
// TODO this is a bit wasteful, if we used change detection could deduplicate added,
// and only generate once, but we'd still want a guanratee the system runs immediately
pub fn insert_route_tree(
	ev: On<Insert, PathPattern>,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	tools: Query<ToolQueryItem, Without<RouteHidden>>,
	mut commands: Commands,
) -> Result {
	let root = ancestors.root_ancestor(ev.entity);
	let mut nodes: Vec<ToolNode> = Vec::new();
	// when added via ChildOf, it will not have been added to the Children,
	// so we check this one manually
	if let Ok(item) = tools.get(ev.entity) {
		nodes.push(ToolNode::from_query(item));
	}

	for entity in children_query
		.iter_descendants_inclusive(root)
		// we've already checked this one
		.filter(|entity| *entity != ev.entity)
	{
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

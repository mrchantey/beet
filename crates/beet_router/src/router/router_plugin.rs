use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "std")]
use beet_ui::prelude::*;

/// Plugin that registers route-building observers for actions.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor whenever
/// actions are spawned in an entity hierarchy. The route-building observers are
/// shared across std and no_std; the std build additionally wires the scene /
/// asset / charcell rendering pipeline and the reflect registrations the
/// help/scene routes and `template_serde`/scripting need (all std-only). Scene
/// routes register as actions (via [`PageRoot`] + [`ActionMeta`]), so there is
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
				// the server model: routers and servers go together, so a server
				// spread on a router boots when the boot fan-out reaches it.
				// `ServerPlugin` installs the `HttpServer` backend and registers the
				// server types.
				.init_plugin::<ServerPlugin>()
				// template routes render through the charcell layout/paint
				// pipeline; without it the `PostParseTree` schedule has no systems
				// and ANSI output is blank.
				.init_plugin::<CharcellPlugin>()
				// per-request route content is built through the template
				// substrate (`spawn_template`), which needs the template plugins.
				.init_plugin::<TemplatePlugin>()
				.init_plugin::<DocumentPlugin>()
				// the re-entrant stack of request-scoped render contexts the
				// layout middleware pushes onto and layout widgets read the top of.
				.init_resource::<RequestContextStack>()
				// the app-wide color scheme `SiteLayout` reads: init here so a router
				// app renders the shipped layout without `MaterialStylePlugin` (which
				// also inits it; `init_resource` is idempotent).
				.init_resource::<Theme>()
				.register_type::<HelpHandler>()
				.register_type::<NavigateHandler>()
				// the diagnostic pages: the help/not-found route list and the
				// navigation-failure error page, both rendered through the layout.
				.register_template::<RouteList>()
				.register_template::<ErrorPage>()
				// per-route metadata, bindable via the reserved ref, eg
				// `@entity:PageRoot::ArticleMeta.title`
				.register_type::<ArticleMeta>()
				// the package resource, bindable as eg `@res:PackageConfig.title`
				.register_type::<PackageConfig>()
				// the no-code render-diagnostics config, patchable from markup
				// like `PackageConfig`, eg `<RenderDiagnostics unknown_class="Off"/>`
				.register_type::<RenderDiagnostics>()
				// the no-code site surface: markup-resolved router components
				// (`<RoutesDir/>`, a `BsxLayout` spread) and the by-name
				// route-aware head/sidebar widgets.
				.register_type::<BsxLayout>()
				.register_template::<RouteHead>()
				.register_template::<RouteSidebar>()
				// the shipped document shell a no-code site wraps its pages in,
				// requested with `<SiteLayout>`.
				.register_template::<SiteLayout>()
				// the default app routes as a markup template, so a no-code BSX
				// site requests them with `<DefaultAppRoutes/>`.
				.register_template::<DefaultAppRoutes>();
			// the markup-resolved `<RoutesDir src=".."/>` is registered on every std
			// target so a no-code site loads. Native scans the store at spawn time
			// via the blocking `spawn_routes_dir` observer; wasm (which cannot block)
			// registers only the type and awaits `spawn_routes_dir_async` post-build.
			app.register_type::<RoutesDir>();
			#[cfg(not(target_arch = "wasm32"))]
			app.add_observer(spawn_routes_dir)
				// the no-code static-asset mount, eg `<BlobStoreRoute src="assets"/>`:
				// a template that expands to a blob-store-backed serve route.
				.register_template::<BlobStoreRoute>();
			// the server-to-client websocket channel and the dev-mode live
			// reload watcher, plus its by-name `<LiveReloadScript/>` widget. The
			// channel rides the main HTTP port: `default_router` wires the
			// `/__client_io` upgrade route and `adopt_client_io_socket` adopts the
			// landed `Socket`.
			#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
			app.add_observer(adopt_client_io_socket)
				.add_observer(broadcast_to_clients)
				.add_observer(start_live_reload)
				.add_observer(reload_site_on_change)
				.register_template::<LiveReloadScript>();
			#[cfg(feature = "template_serde")]
			app.add_observer(rebuild_route_trees_on_load);
			// the `<Template src>` include handler (local-file includes resolved
			// against the `SiteRoot`), into the BSX tag seam.
			#[cfg(all(feature = "bsx", feature = "template_serde"))]
			register_template_include(app.world_mut());
			// the live-TUI server, declarable in a router markup spread
			// (`<Router {(TuiServer, ..)}>`); its `on_add` hook boots the
			// terminal app when the boot fan-out selects `tui`.
			#[cfg(feature = "tui")]
			app.register_type::<TuiServer>();
			#[cfg(feature = "scripting")]
			app.register_type::<Script<RequestParts, String>>()
				.register_type::<ExchangeOverloadScript<(), String>>()
				.register_type::<ExchangeOverloadScript<
					RequestParts,
					String,
					RequestParts,
					SerdeIntoResponseMarker,
				>>()
				// the markup-friendly `<ScriptRoute path=".." script=".."/>` front-end.
				.register_template::<ScriptRoute>();

			// the `ExchangeScriptElement` console-capturing entry action, so a
			// `<script {ExchangeScriptElement}>` entry resolves it. Native runs through
			// the default backend (quickjs/rhai); wasm runs in the host realm. The
			// request `input` marshals through beet's [`Value`], so it rides the
			// backend-agnostic `scripting` feature, not `json`.
			#[cfg(feature = "scripting")]
			app.register_type::<ExchangeScriptElement>();
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
/// type being [`PageRequest`], detected via [`ActionMeta::output_is`].
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

/// Observer that rebuilds [`RouteTree`] roots after a [`LoadTemplateSerde`],
/// where reflect-driven [`ChildOf`] inserts settle later than [`PathPattern`]
/// and leave per-leaf trees on the wrong ancestors.
///
/// The load trigger fires synchronously once the hierarchy is whole, so each
/// affected root is recomputed exactly once before any async serving begins.
#[cfg(feature = "template_serde")]
pub fn rebuild_route_trees_on_load(
	ev: On<LoadTemplateSerde>,
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

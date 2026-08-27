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
			// the `PackageConfig` a layout binds through
			// `@res:PackageConfig.title`: registered and seeded there, so a
			// routerless app can author one too.
			.init_plugin::<BootstrapPlugin>()
			.add_observer(insert_action_path_and_params)
			.add_observer(insert_path_pattern_for_late_path_partial)
			// the four triggers that dirty a `RouteTree`: a route joining or
			// leaving the tree (`PathPattern` insert/remove) and a route being
			// hidden or unhidden (`RouteHidden` insert/remove). Each just wakes
			// `rebuild_dirty_route_trees`, which resolves what actually changed.
			.add_observer(queue_route_tree_rebuild_on_insert::<PathPattern>)
			.add_observer(queue_route_tree_rebuild_on_remove::<PathPattern>)
			.add_observer(queue_route_tree_rebuild_on_insert::<RouteHidden>)
			.add_observer(queue_route_tree_rebuild_on_remove::<RouteHidden>)
			// `RequireFeatures` (a beet_core component) is enforced here, where
			// dispatch lives: an unmet declaration fails any call at or under
			// it naming the missing features.
			.add_observer(enforce_require_features);

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
			.register_type::<CacheHeaders>()
			.register_type::<CacheHeadersMiddleware>()
			.register_type::<CorsHandler>()
			.register_type::<CorsConfig>()
			.register_type::<HtmlStoreAction>()
			.register_type::<Router>()
			// a child-sequenced route is `<Route path=".." {ExchangeSequence}>` with
			// the steps as direct children (the sequence reads its direct children).
			.register_type::<ExchangeSequence>();

		// std-only: the scene/asset/charcell rendering pipeline (help pages,
		// markdown/html scenes → ANSI/text) and the reflect registrations for
		// the help/navigate middleware, which live in the std-only `help` /
		// `navigate` render-media modules. no_std routers dispatch and fall back
		// to plain text without any of this.
		#[cfg(feature = "std")]
		{
			app
				// store types + the store-path resolution observers (`DirPath` /
				// `BlobPath` -> scoped `BlobStore` / `Blob`), which `RoutesDir` and
				// `ServeBlobs` resolve by ancestry.
				.init_plugin::<StorePlugin>()
				// the server model: routers and servers go together, so a server
				// spread on a router starts when its entity's start walk reaches it.
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
				.register_template::<DefaultAppRoutes>()
				// the standard blob-store agent toolset, for agent scenes (eg a
				// thread's `<StoreToolset/>`); the store itself is mounted with a
				// plain `{FsStore{path:..}}`.
				.register_template::<StoreToolset>();
			// the markup-resolved `<RoutesDir src=".."/>`, registered on every std
			// target so a no-code site loads. Its discovery observer scans the store
			// asynchronously (off the runtime, see `RoutesDir::spawn_on_insert`), so it
			// runs on wasm too rather than needing a separate blocking/async split.
			app.register_type::<RoutesDir>()
				.add_observer(RoutesDir::spawn_on_insert);
			// the markup-resolved `<TemplateDir src="templates"/>`: its insert
			// observer reads the dir through the nearest ancestor `BlobStore` and
			// registers each `.bsx`/`.js` template by module path, off the runtime
			// (so it runs on wasm too). An entry's own template dirs are also
			// pre-scanned synchronously by the cli before the entry parses, so
			// entry-level tags like `<Styles/>` resolve.
			app.register_type::<TemplateDir>()
				.add_observer(TemplateDir::register_on_insert);
			// the entry-declared store root (`<StoreRoot src="../.."/>`), read by
			// entry resolution before the store builds; inert in the built tree.
			app.register_type::<StoreRoot>();
			// the no-code static-asset mount: `ServeBlobs` owns its mount prefix and
			// inserts its own greedy capture + handler, serving from the nearest
			// self-or-ancestor store, eg `<AssetsDir src="assets"/>`.
			// Cross-platform, so the wasm Worker resolves a served site's asset routes.
			app.register_template::<Route>()
				// the persistent page route (`<Route path="/" {FixedPage}>`): its
				// declared children are one live tree served by every request.
				.register_type::<FixedPage>()
				.register_template::<ServeBlobs>()
				.register_type::<ServeBlobsHandler>()
				// the markup-declared directory mount (`<AssetsDir src=.. prefix=..>`):
				// `ServeBlobs` scoped to a subdir of the inherited store.
				.register_template::<AssetsDir>()
				// the browser-wasm page templates: the module loader and the program
				// reference a served page boots a wasm `beet` binary with.
				.register_template::<Wasm>()
				.register_template::<MainBsx>();
			// the server-to-client websocket channel and the dev-mode live
			// reload watcher, plus its by-name `<LiveReloadScript/>` widget. The
			// channel rides the main HTTP port: `Router::with_defaults` wires the
			// `/__client_io` upgrade route and `adopt_client_io_socket` adopts the
			// landed `Socket`.
			#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
			app.add_observer(adopt_client_io_socket)
				.add_observer(broadcast_to_clients)
				.add_observer(start_live_reload)
				.add_observer(reload_site_on_change)
				.add_systems(
					Update,
					process_live_reloads
						.run_if(any_with_component::<NeedsReload>),
				)
				.register_template::<LiveReloadScript>();
			// where client_io is compiled out (wasm Worker, no-dev-reload builds)
			// mark `<LiveReloadScript/>` as a known featured-out tag, so a site
			// layout that includes it still loads and renders nothing rather than
			// failing template resolution.
			#[cfg(not(all(
				feature = "client_io",
				not(target_arch = "wasm32")
			)))]
			app.allow_unregistered("LiveReloadScript");
			#[cfg(feature = "template_serde")]
			app.add_observer(rebuild_route_trees_on_load);
			// the `<Template src>` include handler (local-file includes resolved
			// against the nearest ancestor `BlobStore`), into the BSX tag seam.
			#[cfg(all(feature = "bsx", feature = "template_serde"))]
			register_template_include(app.world_mut());
			// the live-TUI server, declarable in a router markup spread
			// (`<Router {(TuiServer, ..)}>`); its `on_add` hook boots the
			// terminal app when the start walk selects `tui`.
			#[cfg(feature = "tui")]
			app.register_type::<TuiServer>();
			// the multi-tenant SSH-TUI server, likewise declarable in a markup
			// spread. Registered by feature rather than by `SshTuiPlugin` (which
			// adds the per-connection behavior), so an entry naming it resolves in
			// any binary that linked the transport.
			#[cfg(feature = "ssh")]
			app.register_type::<SshTuiServer>();
			#[cfg(feature = "scripting")]
			app.register_type::<Script<RequestParts, String>>()
				.register_type::<ExchangeScript<(), String>>()
				.register_type::<ExchangeScript<
					RequestParts,
					String,
					RequestParts,
					SerdeIntoResponseMarker,
				>>()
				// the markup-friendly `<ScriptRoute path=".." script=".."/>` front-end.
				.register_template::<ScriptRoute>();

			// the `ExchangeScriptElement` console-capturing entry action, so a
			// `<script {ExchangeScriptElement}>` entry resolves it. The backend it
			// runs on is `Script`'s compile-time choice, so this registration rides
			// the backend-agnostic `scripting` feature.
			#[cfg(feature = "scripting")]
			app.register_type::<ExchangeScriptElement>();

			// cross-transport analytics: the request-middleware type is serde/std,
			// registered so `<SiteAnalytics/>` can author it. The storage +
			// persistence observer need the json store surface, and are inert until
			// an `<AnalyticsConfig/>` is spawned, so nothing records until a site
			// opts in with that on-switch.
			app.register_type::<AnalyticsMiddleware>();
			#[cfg(feature = "json")]
			app.add_plugins(analytics_plugin);
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
fn insert_action_path_and_params(
	ev: On<Insert, ActionMeta>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	mut commands: Commands,
) -> Result {
	// only entities that have their own PathPartial become routes; children of a
	// route (eg sequence steps) have no PathPartial themselves, and a url space
	// root (a `Router`) bounds its subtree rather than being a route itself.
	if !paths.get(ev.entity).is_ok_and(|path| !path.is_root) {
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
fn insert_path_pattern_for_late_path_partial(
	ev: On<Insert, PathPartial>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	actions: Query<(), (With<ActionMeta>, Without<PathPattern>)>,
	mut commands: Commands,
) -> Result {
	// ActionMeta must already be present, PathPattern not yet computed, and a url
	// space root is not itself a route (see `insert_action_path_and_params`).
	if !actions.contains(ev.entity)
		|| paths.get(ev.entity).is_ok_and(|path| path.is_root)
	{
		return Ok(());
	}
	let path = PathPattern::collect(ev.entity, &ancestors, &paths)?;
	let params = ParamsPattern::collect(ev.entity, &ancestors, &params)?;
	commands.entity(ev.entity).insert((path, params));
	Ok(())
}

/// Observer that wakes [`rebuild_dirty_route_trees`] whenever `T` is inserted
/// on any entity: a route joining the tree ([`PathPattern`]) or a route being
/// hidden ([`RouteHidden`]).
///
/// The wake is deferred (queued rather than run inline), so by the time it
/// runs the insert has settled — in particular so a route added via a bundle's
/// `ChildOf` has already landed in its parent's [`Children`], which is not
/// guaranteed yet at the point this observer itself fires.
fn queue_route_tree_rebuild_on_insert<T: Component>(
	_ev: On<Insert, T>,
	mut commands: Commands,
) {
	queue_route_tree_rebuild(&mut commands);
}

/// Observer that wakes [`rebuild_dirty_route_trees`] whenever `T` is removed
/// from any entity, including via despawn: a route leaving the tree
/// ([`PathPattern`]) or a hidden route being unhidden ([`RouteHidden`]).
///
/// `On<Remove, T>` fires *before* the component actually leaves the entity, so
/// resolving the dirty namespace here (rather than in the woken system, which
/// runs later) would still see `T` present; the wake is deferred for the same
/// reason as the insert half, and the reconciler itself works out what
/// changed via [`RemovedComponents`].
fn queue_route_tree_rebuild_on_remove<T: Component>(
	_ev: On<Remove, T>,
	mut commands: Commands,
) {
	queue_route_tree_rebuild(&mut commands);
}

/// Queue one run of [`rebuild_dirty_route_trees`], deferred past whatever
/// structural change triggered it.
fn queue_route_tree_rebuild(commands: &mut Commands) {
	commands.queue(|world: &mut World| {
		if let Ok(Err(err)) = world.run_system_cached(rebuild_dirty_route_trees)
		{
			world.handle_command_error::<RouteTree>(err);
		}
	});
}

/// Recomputes every dirty [`RouteTree`] namespace: the one grouping walk
/// ([`RouteTree::rebuild_subtree`]) that every trigger in this module funnels
/// into, run once per dirty namespace root rather than once per changed
/// entity.
///
/// A live [`PathPattern`]/[`RouteHidden`] insert or mutation dirties its own
/// enclosing namespace. A removal dirties that namespace too, *if* the entity
/// is still resolvable — a live component removal leaves the entity and its
/// ancestry intact, so [`PathPattern::namespace_root`] still answers. A full
/// despawn does not: by the time this runs the entity is gone, so which
/// namespace lost the route can no longer be found, and every existing
/// namespace is rebuilt this pass instead ([`RemovedComponents`] is what
/// makes despawn visible at all here; a plain [`Changed`] query cannot).
fn rebuild_dirty_route_trees(
	changed: Query<Entity, Or<(Changed<PathPattern>, Changed<RouteHidden>)>>,
	mut removed_paths: RemovedComponents<PathPattern>,
	mut removed_hidden: RemovedComponents<RouteHidden>,
	all_entities: Query<Entity>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	children_query: Query<&Children>,
	actions: Query<ActionQueryItem, Without<RouteHidden>>,
	existing_trees: Query<Entity, With<RouteTree>>,
	mut commands: Commands,
) -> Result {
	let mut dirty: HashSet<Entity> = HashSet::new();
	let mut sweep_all = false;

	for entity in changed.iter() {
		dirty.insert(PathPattern::namespace_root(entity, &ancestors, &paths));
	}
	// a removed component leaves the entity itself in place unless it was the
	// despawn that took it with it; `all_entities` (an unfiltered `Query<Entity>`)
	// is the only reliable liveness check, since `ancestors`/`paths` fail alike
	// for "gone" and for "alive but parentless"/"alive but no PathPartial".
	for entity in removed_paths.read().chain(removed_hidden.read()) {
		if all_entities.contains(entity) {
			dirty.insert(PathPattern::namespace_root(entity, &ancestors, &paths));
		} else {
			sweep_all = true;
		}
	}
	if sweep_all {
		dirty.extend(existing_trees.iter());
	}

	for root in dirty {
		RouteTree::rebuild_subtree(
			root,
			&ancestors,
			&paths,
			&children_query,
			&actions,
			&existing_trees,
			&mut commands,
		)?;
	}
	Ok(())
}

/// Observer that rebuilds [`RouteTree`] roots after a [`LoadTemplateSerde`],
/// where reflect-driven [`ChildOf`] inserts settle later than [`PathPattern`]
/// and leave per-leaf trees on the wrong ancestors — invisible to
/// [`rebuild_dirty_route_trees`], which watches component changes, not
/// reparenting.
///
/// The load trigger fires synchronously once the hierarchy is whole, so each
/// affected namespace is recomputed exactly once, through the same
/// [`RouteTree::rebuild_subtree`] walk, before any async serving begins.
#[cfg(feature = "template_serde")]
fn rebuild_route_trees_on_load(
	ev: On<LoadTemplateSerde>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	children_query: Query<&Children>,
	actions: Query<ActionQueryItem, Without<RouteHidden>>,
	existing_trees: Query<Entity, With<RouteTree>>,
	mut commands: Commands,
) -> Result {
	// collect unique namespace roots so we rebuild each tree at most once. A
	// nested `Router` is its own url space, so the rebuild must land per
	// namespace, not once on the document root.
	let mut roots: Vec<Entity> = ev
		.entities
		.iter()
		.map(|entity| PathPattern::namespace_root(*entity, &ancestors, &paths))
		.collect();
	roots.sort();
	roots.dedup();

	for root in roots {
		RouteTree::rebuild_subtree(
			root,
			&ancestors,
			&paths,
			&children_query,
			&actions,
			&existing_trees,
			&mut commands,
		)?;
	}
	Ok(())
}

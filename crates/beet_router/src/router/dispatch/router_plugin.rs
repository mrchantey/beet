use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "scene")]
use beet_scene::prelude::*;
#[cfg(feature = "std")]
use beet_ui::prelude::*;

/// Plugin that registers route-building observers for actions.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor whenever
/// actions are spawned in an entity hierarchy. The route-building observers are
/// shared across std and no_std; the std build additionally wires the scene /
/// asset / charcell rendering pipeline and the reflect registrations the
/// help/scene routes and `scene`/scripting need (all std-only). Scene
/// routes register as actions (via [`PageRoot`] + [`ActionMeta`]), so there is
/// no separate scene observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		// the table wire protocol over a mounted route (`<ServeTables
		// prefix="tables"/>`), the table twin of `ServeBlobs`
		#[cfg(feature = "json")]
		app.install_types::<ServeTables>()
			.register_type::<ServeTablesHandler>();
		app.install_types::<(
			ActionPlugin,
			AsyncPlugin,
			// the `PackageConfig` a layout binds through
			// `@res:PackageConfig.title`: registered and seeded there, so a
			// routerless app can author one too.
			BootstrapPlugin,
		)>()
		.add_observer(insert_action_path)
		.add_observer(insert_path_pattern_for_late_path_partial)
		// the eight triggers that dirty a `RouteTree`: a url space root
		// appearing (a root `PathPartial`, so a routeless router still owns
		// a tree), a route joining or leaving the tree (`PathPattern`
		// insert/remove), a route being hidden or unhidden (`RouteHidden`
		// insert/remove), a reader declaring its params (a `RouteParams` or
		// `SubtreeParams` insert, which can land after its route joined
		// the tree, ie a `children!` step spawned behind the route), and a
		// template build settling its slots (`SpawnTemplate`, which
		// reparents routes without touching any of those). Each just wakes
		// `rebuild_dirty_route_trees`, which resolves what actually changed.
		.add_observer(queue_route_tree_rebuild_on_root)
		.add_observer(queue_route_tree_rebuild_on_insert::<PathPattern>)
		.add_observer(queue_route_tree_rebuild_on_remove::<PathPattern>)
		.add_observer(queue_route_tree_rebuild_on_insert::<RouteHidden>)
		.add_observer(queue_route_tree_rebuild_on_remove::<RouteHidden>)
		.add_observer(queue_route_tree_rebuild_on_insert::<RouteParams>)
		.add_observer(queue_route_tree_rebuild_on_insert::<SubtreeParams>)
		.add_observer(rebuild_route_tree_on_build)
		// a replaced route outlives the swap while a request holds it
		.add_systems(Update, despawn_idle_retired);
		// documents read as scenes, and a loaded scene's routes settle their
		// parents after the load, on every target that loads scenes (the esp
		// scene server among them).
		#[cfg(feature = "scene")]
		app.install_types::<ScenePlugin>()
			.add_observer(rebuild_route_trees_on_load);
		// `CfgExcluded` (a beet_core component) is reported here, where
		// dispatch lives: an unmet declaration fails any call at or under it
		// naming the missing features.
		#[cfg(feature = "scene")]
		app.add_observer(report_cfg_excluded);

		// no_std-core reflect registrations: these types are shared across std
		// and no_std and reflection works on bare metal, so register them
		// unconditionally to keep scene-loading reflection available on no_std.
		// `register_type` initialises the `AppTypeRegistry` if the app has not
		// added one, so this is safe without an explicit registry.
		app.register_type::<InterruptOnRun>()
			.register_type::<InterruptOnEnd>()
			.register_type::<PathPartial>()
			.register_type::<PathPattern>()
			// what `--help` says a route does, for the routes that share a
			// handler and differ by what they carry
			.register_type::<RouteDescription>()
			.register_type::<RequestLogger>()
			.register_type::<NoCacheHeaders>()
			.register_type::<CacheHeaders>()
			.register_type::<CacheHeadersMiddleware>()
			.register_type::<CorsHandler>()
			.register_type::<CorsConfig>()
			// the strict url space, refusing a param its route does not take
			.register_type::<RejectUnknownParams>()
			.register_type::<Router>()
			// a child-sequenced route is `<Route path=".." {ExchangeSequence}>` with
			// the steps as direct children (the sequence reads its direct children).
			.register_type::<ExchangeSequence>()
			// the explicit fan-out route (`<Route path="all" {RouteMatrix{..}}>`):
			// the matrix itself, the per-verb child route it spawns, and that
			// child's dispatch action.
			.register_type::<RouteMatrix>()
			.register_type::<RouteMatrixStep>();

		// std-only: the scene/asset/charcell rendering pipeline (help pages,
		// markdown/html scenes → ANSI/text) and the reflect registrations for
		// the help/navigate middleware, which live in the std-only `help` /
		// `navigate` render-media modules. no_std routers dispatch and fall back
		// to plain text without any of this.
		#[cfg(feature = "std")]
		{
			app
				// every beet_net registration: the store types + the store-path
				// resolution observers (`DirPath` / `BlobPath` -> scoped
				// `BlobStore` / `Blob`), which `RoutesDir` and `ServeBlobs`
				// resolve by ancestry, and the vault declarations.
				.install_types::<(
					NetPlugin,
					// the server model: routers and servers go together, so a server
					// spread on a router starts when its entity's start walk reaches it.
					// `ServerPlugin` installs the `HttpServer` backend and registers the
					// server types.
					ServerPlugin,
					// every document a route serves parses through the parser
					// registry, and every scene route renders through the render
					// target registry
					ParsePlugin,
					RenderPlugin,
				)>();
			// the Office formats a route serves and writes
			#[cfg(feature = "ooxml")]
			app.install_types::<OoxmlPlugin>();
			// a router a `CardDeck` marks steps as a stack of cards
			app.install_types::<CardStackPlugin>();
			app
				// the scene routes' render params and the media a request resolves
				.register_type::<RenderRoot>()
				.register_type::<MediaIngestPolicy>()
				// template routes render through the charcell layout/paint
				// pipeline; without it the `PostParseTree` schedule has no systems
				// and ANSI output is blank.
				.install_types::<(
					CharcellPlugin,
					// per-request route content is built through the template
					// substrate (`spawn_template`), which needs the template plugins.
					TemplatePlugin,
					DocumentPlugin,
					// a page, a layout or an entry instantiating a document
					SceneStorePlugin,
				)>()
				// the re-entrant stack of request-scoped render contexts the
				// layout middleware pushes onto and layout widgets read the top of.
				.init_resource::<RequestContextStack>()
				// the app-wide color scheme `SiteLayout` reads: init here so a router
				// app renders the shipped layout without `MaterialStylePlugin` (which
				// also inits it; `init_resource` is idempotent).
				.init_resource::<Theme>()
				.register_type::<HelpHandler>()
				.register_type::<NavigateHandler>()
				// the color scheme a page is themed by, read leniently by
				// `PageClasses` beneath every extractor rendering through it
				.install(ColorSchemeParams::declare_on::<HelpHandler>())
				// the diagnostic pages: the help/not-found route list and the
				// navigation-failure error page, both rendered through the layout.
				.install_types::<(RouteList, NotFoundPage, ErrorPage)>()
				// per-route metadata, bindable via the reserved ref, eg
				// `@entity:PageRoot::DocMeta.title`
				.register_type::<DocMeta>()
				// the no-code render-diagnostics config, patchable from markup
				// like `PackageConfig`, eg `<RenderDiagnostics unknown_class="Off"/>`
				.register_type::<RenderDiagnostics>()
				// the no-code site surface: markup-resolved router components
				// (`<RoutesDir/>`, a `Layout` spread) and the by-name
				// route-aware head/sidebar widgets.
				.register_type::<Layout>()
				.register_type::<ServerIsland>()
				.register_type::<ServerIslandsRoute>()
				// the declared icon set `RouteHead` links, deriving its own files,
				// and the social cards it points each page's preview at
				.register_type::<IconSet>()
				.install_types::<SocialCards>()
				.install(ColorSchemeParams::declare_on::<Layout>())
				.install_types::<(
					RouteHead,
					RouteSidebar,
					// the generated page listing a content index page places instead
					// of a hand-maintained list, eg `<RouteIndex reverse="true"/>`
					RouteIndex,
					// the article chrome a blog layout places above the post body,
					// rendered from the page's own `DocMeta`
					ArticleHeader,
					YouTubeEmbed,
					// the shipped document shell a no-code site wraps its pages in,
					// requested with `<SiteLayout>`.
					SiteLayout,
					// its chrome-free twin, for an app that wants the theme and
					// nothing else: `<Router {Layout(Template("AppShell"))}>`.
					AppShell,
					// the machine-readable faces of a site, each an opt-in static
					// route scoped by where it is declared
					Robots,
					Sitemap,
					RssFeed,
					// the island route, declared once at a router's root, and the
					// live region of a page it answers
					ServerIslands,
					// the default app routes as a markup template, so a no-code BSX
					// site requests them with `<DefaultAppRoutes/>`.
					DefaultAppRoutes,
					// the standard blob-store agent toolset, for agent scenes (eg a
					// thread's `<StoreToolset/>`); the store itself is mounted with a
					// plain `{FsStore{path:..}}`.
					StoreToolset,
				)>();
			// the search index is a json document, so it rides the format's
			// feature
			#[cfg(feature = "json")]
			app.install_types::<SearchIndex>();
			// the markup-resolved `<RoutesDir src=".."/>`, registered on every std
			// target so a no-code site loads. Its discovery observer scans the store
			// asynchronously (off the runtime, see `RoutesDir::spawn_on_insert`), so it
			// runs on wasm too rather than needing a separate blocking/async split.
			app.register_type::<RoutesDir>()
				.add_observer(RoutesDir::spawn_on_insert)
				.add_systems(
					Update,
					(refresh_changed_routes, rescan_changed_dirs::<RoutesDir>)
						.in_set(BlobReactions),
				);
			// the declarations that act before the entry builds (see `Prescan`),
			// in preload order: the entry-declared store root (`<PackageRoot
			// src="../.."/>`) first, so nothing runs against a store it has not
			// rebased, then the crate check and the secrets load.
			app.install_types::<(PackageRoot, RequireCfg)>();
			// the declaration resolves in every build, so an entry loaded by
			// a binary that never reads secrets (a cloud box's) builds it
			// inert rather than warning it unregistered; `vault` adds the
			// prescan load
			app.register_type::<Secrets>();
			#[cfg(feature = "vault")]
			app.install_types::<Secrets>();
			// the no-code static-asset mount: `ServeBlobs` owns its mount prefix and
			// inserts its own greedy capture + handler, serving from the nearest
			// self-or-ancestor store, eg `<AssetsDir src="assets"/>`.
			// Cross-platform, so the wasm Worker resolves a served site's asset routes.
			app.install_types::<(
				Route,
				// the renamed-page redirect (`<Redirect path="post-1"
				// redirect="full-stack-bevy"/>`), an ordinary GET route
				// answering a 301
				Redirect,
			)>()
			// the redirect's target as plain data, read by dispatch and by
			// the static export's meta-refresh stub
			.register_type::<RedirectTo>()
			// the document-field route (`<FieldRoute path=".." field=".."
			// verb="push"/>`): a typed document action plus the
			// `ExchangeOverload` that markup cannot spread for itself.
			.install_types::<FieldRoute>()
			.register_type::<FieldVerb>()
			// the persistent page route (`<Route path="/" {FixedPage}>`): its
			// declared children are one live tree served by every request.
			.register_type::<FixedPage>()
			.install_types::<ServeBlobs>()
			.register_type::<ServeBlobsHandler>()
			// one store file served as a page (`<Route path="guide"
			// {BlobPage{path:"guide.md"}}/>`)
			.register_type::<BlobPage>()
			// any document in the store a request names, rendered as it
			// accepts (`<DocumentView/>`, `view docs/plan.docx`)
			.register_type::<DocumentView>()
			// the markup-declared directory mount (`<AssetsDir src=.. prefix=..>`):
			// `ServeBlobs` scoped to a subdir of the inherited store.
			.install_types::<(
				AssetsDir,
				// the browser-wasm page template: the module loader plus the launch
				// a served page boots a wasm `beet` binary with.
				Wasm,
			)>();
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
				// after the reactions, so the guards a change parked defer the
				// dispatch until its re-reads and rescans settle
				.add_systems(
					Update,
					process_live_reloads
						.run_if(any_with_component::<NeedsReload>)
						.after(BlobReactions),
				)
				.install_types::<LiveReloadScript>();
			// the site widgets' own rules (the app shell's scroll region, the
			// index heading link), extending the shared rule set the way the
			// card-stack rules do, so they compose with the material set and
			// override the bare tag rules on a tie.
			app.world_mut()
				.get_resource_or_init::<RuleSet>()
				.extend_rules(app_shell_rules())
				.extend_rules(route_index_rules());
			// the browser's server, declarable beside the others
			// (`<CallOnReady {(HttpServer, TuiServer, DomServer, ..)}>`) on every
			// target so an entry keeps its shape; its facet boots the DOM host
			// when the start walk selects `dom`, in a browser tab only.
			app.register_type::<DomServer>();
			// the live-TUI server, declarable in a router markup spread
			// (`<Router {(TuiServer, ..)}>`); its `on_add` hook boots the
			// terminal app when the start walk selects `tui`.
			#[cfg(feature = "tui")]
			app.register_type::<TuiServer>().install((
				SubtreeParams::declare_on::<
					TuiServer,
					(BootParams, ServerParams),
				>(),
				ColorSchemeParams::declare_on::<TuiServer>(),
			));
			// the multi-tenant SSH-TUI server, likewise declarable in a markup
			// spread. Registered by feature rather than by `SshTuiPlugin` (which
			// adds the per-connection behavior), so an entry naming it resolves in
			// any binary that linked the transport.
			#[cfg(feature = "ssh")]
			app.register_type::<SshTuiServer>().install(
				SubtreeParams::declare_on::<
					SshTuiServer,
					(BootParams, ServerParams),
				>(),
			);
			// the scripted route marker in the one instantiation `<ScriptRoute>`
			// builds: the whole request in, whatever the script returned out.
			// `Script<Value, Value>` itself is registered upstream by
			// `ActionPlugin`, the sole registered instantiation, so base-name tag
			// resolution stays unambiguous.
			#[cfg(feature = "scripting")]
			app.register_type::<ExchangeScript<
				Value,
				Value,
				ScriptInputMarker,
				ScriptAnswerMarker,
			>>()
			// the markup-friendly `<ScriptRoute path=".." script=".."/>` front-end.
			.install_types::<ScriptRoute>();

			// the `ExchangeScriptElement` console-capturing entry action, so a
			// `<script {ExchangeScriptElement}>` entry resolves it. The backend it
			// runs on is `Script`'s compile-time choice, so this registration rides
			// the backend-agnostic `scripting` feature.
			#[cfg(feature = "scripting")]
			app.register_type::<ExchangeScriptElement>();
			// files a script derives from resources, ie the logo svgs
			#[cfg(feature = "scripting")]
			app.register_type::<ScriptDerivation>();

			// cross-transport analytics: the request-middleware type is serde/std,
			// registered so `<AnalyticsMiddleware/>` can author it. The storage +
			// persistence observer need the json store surface, and are inert until
			// an `<AnalyticsConfig/>` is spawned, so nothing records until a site
			// opts in with that on-switch.
			app.register_type::<AnalyticsMiddleware>();
			// the publication declaration, its index and the well-known route
			// it serves
			#[cfg(feature = "dag_cbor")]
			app.install_types::<StandardSitePub>();
			// an article's comments and the link to reply, registered with
			// the records so a build without `atproto` fails the comments
			// island over to its fallback rather than swapping in nothing
			#[cfg(feature = "dag_cbor")]
			app.install_types::<(StandardSiteComments, StandardSiteReplyLink)>(
			);
			// the publish sequence's steps, which write to a public repo and fetch
			// the live site; the route declaring them is gated on the same
			// feature, so no build loads them inert
			#[cfg(feature = "atproto")]
			app.register_type::<StandardSitePublish>()
				.register_type::<StandardSiteAnnounce>()
				.register_type::<StandardSiteIndexWrite>()
				.register_type::<StandardSiteProbe>();
			// the optional standard site `content` formats, Leaflet today
			#[cfg(feature = "standard_site_extra")]
			app.install((LeafletRenderer, LeafletParser::all()));
			#[cfg(feature = "json")]
			app.add_plugins(analytics_plugin);
			// the visit count a server island reads back from the analytics
			// store
			#[cfg(feature = "json")]
			app.install_types::<VisitCount>();
		}
	}
}

/// Observer that listens for new actions and inserts their [`PathPattern`].
/// Any [`PathPartial`] will be collected so long as it is spawned at the same
/// time as the action, even if it comes after it in the tuple. This is because,
/// unlike OnAdd component hooks, observers run after the entire tree is spawned.
///
/// Only actions whose entity directly carries a [`PathPartial`] become routes.
/// Descendants of a route entity (eg sequence steps) are skipped.
fn insert_action_path(
	ev: On<Insert<ActionMeta>>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	mut commands: Commands,
) -> Result {
	// only entities that have their own PathPartial become routes; children of a
	// route (eg sequence steps) have no PathPartial themselves, and a url space
	// root (a `Router`) bounds its subtree rather than being a route itself.
	if !paths.get(ev.entity).is_ok_and(|path| !path.is_root) {
		return Ok(());
	}
	commands
		.entity(ev.entity)
		.insert(PathPattern::collect(ev.entity, &ancestors, &paths)?);
	Ok(())
}

/// Observer that catches the scene-load case where [`PathPartial`] is
/// inserted *after* [`ActionMeta`], so [`insert_action_path`] would have
/// short-circuited. Re-runs the path collection here.
fn insert_path_pattern_for_late_path_partial(
	ev: On<Insert<PathPartial>>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	actions: Query<(), (With<ActionMeta>, Without<PathPattern>)>,
	mut commands: Commands,
) -> Result {
	// ActionMeta must already be present, PathPattern not yet computed, and a url
	// space root is not itself a route (see `insert_action_path`).
	if !actions.contains(ev.entity)
		|| paths.get(ev.entity).is_ok_and(|path| path.is_root)
	{
		return Ok(());
	}
	commands
		.entity(ev.entity)
		.insert(PathPattern::collect(ev.entity, &ancestors, &paths)?);
	Ok(())
}

/// Observer that wakes [`rebuild_dirty_route_trees`] when a url space root
/// appears, a [`PathPartial::root`] (every `Router`). The root is not itself a
/// route, so no [`PathPattern`] insert ever fires for it; without this wake a
/// router with no routes would never get a tree, and every request into it,
/// `--help` included, would fail dispatch rather than answer not-found.
fn queue_route_tree_rebuild_on_root(
	ev: On<Insert<PathPartial>>,
	paths: Query<&PathPartial>,
	mut commands: Commands,
) {
	if paths.get(ev.entity).is_ok_and(|path| path.is_root) {
		queue_route_tree_rebuild(&mut commands, ev.entity);
	}
}

/// Observer that wakes [`rebuild_dirty_route_trees`] whenever `T` is inserted
/// on any entity: a route joining the tree ([`PathPattern`]), a route being
/// hidden ([`RouteHidden`]) or a reader declaring its params ([`RouteParams`],
/// [`SubtreeParams`]).
///
/// The wake is deferred (queued rather than run inline), so by the time it
/// runs the insert has settled — in particular so a route added via a bundle's
/// `ChildOf` has already landed in its parent's [`Children`], which is not
/// guaranteed yet at the point this observer itself fires.
fn queue_route_tree_rebuild_on_insert<T: Component>(
	ev: On<Insert<T>>,
	mut commands: Commands,
) {
	queue_route_tree_rebuild(&mut commands, ev.entity);
}

/// Observer that wakes [`rebuild_dirty_route_trees`] whenever `T` is removed
/// from any entity, including via despawn: a route leaving the tree
/// ([`PathPattern`]) or a hidden route being unhidden ([`RouteHidden`]).
///
/// `On<Remove<T>>` fires *before* the component actually leaves the entity, so
/// resolving the dirty namespace here (rather than in the woken system, which
/// runs later) would still see `T` present; the wake is deferred for the same
/// reason as the insert half, and the reconciler itself works out what
/// changed via [`RemovedComponents`].
///
/// What it does record is the url space the entity sits in while its
/// ancestry is still whole, for a despawn, after which nothing can find it:
/// see [`RebuildWake::namespace`].
fn queue_route_tree_rebuild_on_remove<T: Component>(
	ev: On<Remove<T>>,
	parents: Query<&ChildOf>,
	partials: Query<&PathPartial>,
	alive: Query<Entity>,
	mut commands: Commands,
) {
	let whole = parents
		.iter_ancestors(ev.entity)
		.all(|ancestor| alive.contains(ancestor));
	let namespace = whole
		.then(|| PathPattern::namespace_root(ev.entity, &parents, &partials));
	queue_route_tree_wake(&mut commands, RebuildWake {
		entity: ev.entity,
		namespace,
	});
}

/// What woke a [`rebuild_dirty_route_trees`]: the entity whose route state
/// changed and, for a removal, the url space it sat in.
#[derive(Debug, Clone, Copy)]
struct RebuildWake {
	/// The entity whose [`PathPattern`], [`RouteHidden`], [`RouteParams`] or
	/// [`SubtreeParams`] changed.
	entity: Entity,
	/// The namespace root the entity sat in when its removal was observed,
	/// read while its ancestry was whole. `None` for an insert, and for a
	/// removal whose parent was already gone (a top-down despawn reaching a
	/// child after its parent), which leaves the namespace unknown.
	namespace: Option<Entity>,
}

/// Queue one run of [`rebuild_dirty_route_trees`] for `entity`, deferred past
/// whatever structural change triggered it.
///
/// The triggering entity is passed *in* rather than left to be rediscovered by
/// change detection: several routes commonly join one router in a single
/// command flush, and every wake in that flush shares one change tick, so only
/// the first run would see a [`Changed`] filter answer true. Every later
/// route — in particular the last one added — would then be dropped from the
/// tree with nothing left to wake it again.
fn queue_route_tree_rebuild(commands: &mut Commands, entity: Entity) {
	queue_route_tree_wake(commands, RebuildWake {
		entity,
		namespace: None,
	});
}

/// Queue one run of [`rebuild_dirty_route_trees`] for `wake`.
fn queue_route_tree_wake(commands: &mut Commands, wake: RebuildWake) {
	commands.queue(move |world: &mut World| {
		if let Ok(Err(err)) =
			world.run_system_cached_with(rebuild_dirty_route_trees, wake)
		{
			world.handle_command_error::<RouteTree>(err);
		}
	});
}

/// Recomputes every dirty [`RouteTree`] namespace: the one grouping walk
/// ([`RouteTreeBuilder::rebuild_subtree`]) that every trigger in this module
/// funnels into, run once per dirty namespace root rather than once per
/// changed entity.
///
/// `trigger` names the entity whose [`PathPattern`], [`RouteHidden`],
/// [`RouteParams`] or [`SubtreeParams`] just changed, handed over by the
/// waking observer.
/// An insert or a live removal leaves the entity and its ancestry intact, so
/// [`PathPattern::namespace_root`] answers
/// and only that namespace is dirty. A despawn does not: by the time this runs
/// the entity is gone, so the namespace its removal recorded is dirty, and
/// none at all once that namespace went with it, its tree gone too. A
/// removal that recorded none, its parent already gone when it was observed,
/// rebuilds every existing namespace this pass instead. So a subtree torn
/// down leaves first, ie a thread a route built for one request, rebuilds
/// only the url spaces that survive it.
///
/// The [`Changed`] query is the *mutation* half — a path edited in place fires
/// no insert observer — and is a superset filter only, never the sole source
/// of dirt: several wakes in one flush share one change tick, so every run
/// after the first sees it empty.
fn rebuild_dirty_route_trees(
	trigger: In<RebuildWake>,
	changed: Query<Entity, Or<(Changed<PathPattern>, Changed<RouteHidden>)>>,
	all_entities: Query<Entity>,
	mut builder: RouteTreeBuilder,
) -> Result {
	let mut dirty: HashSet<Entity> = HashSet::new();

	for entity in changed.iter() {
		dirty.insert(builder.namespace_of(entity));
	}
	// `all_entities` (an unfiltered `Query<Entity>`) is the only reliable
	// liveness check, since ancestry-based lookups fail alike for "gone" and
	// for "alive but parentless"/"alive but no `PathPartial`".
	match (all_entities.contains(trigger.entity), trigger.namespace) {
		(true, _) => {
			dirty.insert(builder.namespace_of(trigger.entity));
		}
		// a despawn took the route with it, out of the url space it recorded
		(false, Some(namespace)) => {
			if all_entities.contains(namespace) {
				dirty.insert(namespace);
			}
		}
		// a despawn whose url space is unknown, so sweep every namespace
		(false, None) => dirty.extend(builder.existing_roots()),
	}

	for root in dirty {
		builder.rebuild_subtree(root)?;
	}
	Ok(())
}

/// Observer that rebuilds the [`RouteTree`] of a built template's url space,
/// because SLOT RESOLUTION reparents content after its routes were registered.
///
/// The `PathPattern` insert wake resolves a route's namespace from its
/// ancestry at the moment the pattern lands, and slot content is built under
/// the template ELEMENT and spliced into its [`SlotTarget`] afterwards. A host
/// template that owns its own `Router` (a `Router` child of the bundle rather
/// than an ancestor in the entry) therefore sees its slotted routes register
/// against whatever namespace held them before the splice, so
/// `<Host><Route path="deploy"/></Host>` built a `deploy` route that dispatch
/// could not reach, while the host's own `validate`/`plan` (declared in the
/// same bundle, so never reparented) resolved fine.
///
/// [`SpawnTemplate`] is the post-build boundary and fires exactly once per
/// root, with slots resolved; [`rebuild_dirty_route_trees`] re-buckets every
/// route under the settled ancestry. The one build it cannot speak for is a
/// deferred one (a document instantiated), whose slots settle at the
/// drain; that path already has its own rebuild through
/// [`rebuild_route_trees_on_load`].
///
/// [`SlotTarget`]: beet_core::prelude::SlotTarget
fn rebuild_route_tree_on_build(ev: On<SpawnTemplate>, mut commands: Commands) {
	queue_route_tree_rebuild(&mut commands, ev.entity);
}

/// Observer that rebuilds [`RouteTree`] roots after a [`LoadTemplateSerde`],
/// where reflect-driven [`ChildOf`] inserts settle later than [`PathPattern`]
/// and leave per-leaf trees on the wrong ancestors — invisible to
/// [`rebuild_dirty_route_trees`], which watches component changes, not
/// reparenting.
///
/// The load trigger fires synchronously once the hierarchy is whole, so each
/// affected namespace is recomputed exactly once, through the same
/// [`RouteTreeBuilder::rebuild_subtree`] walk, before any async serving begins.
#[cfg(feature = "scene")]
fn rebuild_route_trees_on_load(
	ev: On<LoadTemplateSerde>,
	mut builder: RouteTreeBuilder,
) -> Result {
	// collect unique namespace roots so we rebuild each tree at most once. A
	// nested `Router` is its own url space, so the rebuild must land per
	// namespace, not once on the document root.
	let mut roots: Vec<Entity> = ev
		.entities
		.iter()
		.map(|entity| builder.namespace_of(*entity))
		.collect();
	roots.sort();
	roots.dedup();

	for root in roots {
		builder.rebuild_subtree(root)?;
	}
	Ok(())
}

#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	#[cfg(feature = "scene")]
	use beet_scene::prelude::*;

	/// REGRESSION: the beet.org box's binary, built without `vault`, warned
	/// `<Secrets>` unregistered on every load of the site entry, which
	/// declares it at its top so a deploy loads the package's document. The
	/// declaration is registered in every build, `vault` adding only its
	/// load.
	#[beet_core::test]
	fn every_build_knows_the_secrets_declaration() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let mut root = world.spawn(Router);
		ParseContext::new(
			&mut root,
			&MediaBytes::new_bsx(r#"<Secrets path="../secrets.toml"/>"#),
		)
		.parse()
		.unwrap();
		let root = root.flush();
		let declared = world
			.query_filtered::<Entity, With<Secrets>>()
			.iter(&world)
			.collect::<Vec<_>>();
		declared.len().xpect_eq(1);
		world
			.entity(declared[0])
			.contains::<Unregistered>()
			.xpect_false();
		world.entity(root).get::<TemplateError>().xpect_none();
	}

	/// The store-file routes are authored in markup, as an entry declares them,
	/// so each must be registered: an unregistered tag builds inert and its
	/// route answers `404`.
	#[beet_core::test]
	async fn registers_the_store_file_routes() {
		let store = BlobStore::temp();
		store
			.insert(&fwd_path!("guide.md"), "# Guide\n".to_owned())
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let mut root = world.spawn((store, Router));
		ParseContext::new(
			&mut root,
			&MediaBytes::new_bsx(
				r#"<Route path="guide" {BlobPage{path:"guide.md"}}/>
					<DocumentView/>"#,
			),
		)
		.parse()
		.unwrap();
		let root = root.flush();
		for (path, expected) in
			[("guide", "# Guide"), ("view/guide.md", "# Guide")]
		{
			world
				.entity_mut(root)
				.exchange(
					Request::get(path).with_header::<header::Accept>(vec![
						MediaType::Markdown,
					]),
				)
				.await
				.unwrap_str()
				.await
				.xpect_contains(expected);
		}
	}
}

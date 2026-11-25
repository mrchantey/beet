//! End-to-end test of the no-code site path (the `beetmania` example's shape):
//! a `main.bsx` entry declaring the router with middleware spreads, runtime
//! route discovery from a content directory, and a BSX layout composing the
//! route-aware widgets (`RouteHead`, `RouteSidebar`).
beet_core::test_main!();

use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use beet_scene::prelude::*;
use beet_ui::prelude::*;

const MAIN_BSX: &str = r#"
<!-- the whole site: middleware as spreads, routes discovered from disk,
the package resource patched from markup -->
<Router {(RequestLogger, Layout(Document("templates/Layout.bsx")))}>
	<PackageConfig title="Beet Test Site" description="markup declared"/>
	<RoutesDir src="routes"/>
	<ServeBlobs prefix="assets" {DirPath("assets")}/>
</Router>
"#;

const LAYOUT_BSX: &str = r#"
<html lang="en">
	<RouteHead/>
	<body>
		<RouteSidebar/>
		<main><slot/></main>
	</body>
</html>
"#;

const CARD_BSX: &str = r#"
<section class="card-filled">
	<h2>{@prop:title}</h2>
	<slot/>
</section>
"#;

const COUNTER_BSX: &str = r#"
<article bx:scope="counter">
	<Instance document="templates/widgets/Card" props={{title:"Counter"}}>
		<p>You have clicked {@doc:count=0} times.</p>
		<button bx:onclick="target.with_field('count', count => count + 1)">More</button>
	</Instance>
</article>
"#;

/// Seed an in-memory repo store (entry, layout template, content routes) so the
/// suite runs storage-agnostic, on wasm too, with no temp-dir concurrency hazard.
async fn site_store() -> BlobStore {
	let store = BlobStore::temp();
	for (path, content) in [
		("main.bsx", MAIN_BSX),
		("templates/Layout.bsx", LAYOUT_BSX),
		("templates/widgets/Card.bsx", CARD_BSX),
		("routes/index.md", "# Home\n\nwelcome"),
		("routes/counter.bsx", COUNTER_BSX),
		// a static asset the `<ServeBlobs prefix="assets"/>` mount streams
		("assets/style.css", "body { color: red; }"),
		(
			"routes/docs/intro.md",
			"+++\ntitle = \"The Intro\"\n\n[RouteMeta]\norder = 1\n+++\n\n# Intro\n\nintro body",
		),
		// the BSX surface of the same metadata: the component itself, spread on
		// the page root rather than written as frontmatter
		(
			"routes/docs/spread.bsx",
			r#"<Fragment {DocMeta{title: "The Spread"}} {RouteMeta{order: 2}}><p>spread body</p></Fragment>"#,
		),
	] {
		store.insert(&RelPath::from(path), content).await.unwrap();
	}
	store
}

/// The example's `main.rs` setup in miniature: plugins + the compile-time
/// package config (the title/description come from `MAIN_BSX`), then build the
/// entry into a root carrying the repo store so the markup-declared `<RoutesDir>`
/// and every document the site instantiates resolve it by ancestry.
async fn spawn_site(world: &mut World) -> Entity {
	world.insert_resource(pkg_config!());
	// the same store-backed load the binary and Worker run: read the entry through
	// the repo store and build into a root carrying that store.
	let store = site_store().await;
	let entry = store.get_media(&RelPath::from("main.bsx")).await.unwrap();
	let template = BsxParser::body(world, entry.as_utf8().unwrap()).unwrap();
	let root = world.spawn(store).id();
	world.entity_mut(root).insert_template(template).unwrap();
	// the `<RoutesDir>` scan is async, so settle it before reading the route tree
	// or serving requests (else routes 404)
	AsyncRunner::settle_async_tasks(world).await;
	root
}

/// Request `path`, negotiating HTML, and return the rendered body.
async fn get(world: &mut World, root: Entity, path: &str) -> String {
	world
		.entity_mut(root)
		.exchange(
			Request::get(path)
				.with_header::<header::Accept>(vec![MediaType::Html]),
		)
		.await
		.unwrap_str()
		.await
}

#[beet_core::test]
async fn entry_components_land_on_root() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	// the entry's root element is the spawned entity itself, with the spread
	// middleware stacked alongside the router
	world.entity(root).contains::<Router>().xpect_true();
	world.entity(root).contains::<RequestLogger>().xpect_true();
	world.entity(root).contains::<Layout>().xpect_true();
	// the markup `<PackageConfig/>` patched the real resource, the unnamed
	// fields keeping their compile-time values
	world
		.resource::<PackageConfig>()
		.title
		.as_str()
		.xpect_eq("Beet Test Site");
	world
		.resource::<PackageConfig>()
		.description
		.as_str()
		.xpect_eq("markup declared");
	world
		.resource::<PackageConfig>()
		.version
		.as_str()
		.xpect_eq(env!("CARGO_PKG_VERSION"));
	// discovered routes assembled into the root's route tree
	let tree = world.entity(root).get::<RouteTree>().unwrap();
	tree.find(&["docs", "intro"]).is_some().xpect_true();
}

/// The markup `<ServeBlobs prefix="assets" {DirPath("assets")}/>`
/// mounts the on-disk `assets/` directory (the `DirPath` scopes the repo store to
/// that subdir, resolved at insert time) and streams files beneath it.
#[beet_core::test]
async fn serve_blobs_serves_assets() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	world
		.entity_mut(root)
		.exchange(Request::get("assets/style.css"))
		.await
		.unwrap_str()
		.await
		.xpect_contains("color: red");
}

/// `<HttpServer port="0"><Router/></HttpServer>`: the http server is declarable
/// from markup, adding a facet to the root's `RunningSet` with the router as its
/// dispatch child, via the reflect element path. The reflect insert adds the
/// facet through its `on_add`, so calling the entity starts it on the backend
/// the dev dependencies compile in, bound to an OS-assigned port.
#[beet_core::test]
async fn http_server_declarable_in_markup() {
	use beet_action::prelude::AsyncEntityActionExt;
	// `RouterPlugin` brings in `ServerPlugin`, so it must not be added again.
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let holder = world.spawn_empty().id();
	let server = spawn_bsx_under(
		&mut world,
		holder,
		"<HttpServer port=\"0\"><Router {RequestLogger}/></HttpServer>",
	);
	world
		.entity(server)
		.get::<HttpServer>()
		.unwrap()
		.port
		.xpect_eq(Some(0));
	// the router is the server's dispatch child, carrying its middleware spread
	let router = world.entity(server).get::<Children>().unwrap()[0];
	world.entity(router).contains::<Router>().xpect_true();
	world
		.entity(router)
		.contains::<RequestLogger>()
		.xpect_true();
	// the start walk boots the declared server: calling the entity's
	// `RunningSet` action reaches the backend. The call is fire-and-forget: the
	// boot never resolves (the server parks on its `Running<Response>`).
	world.run_async_local(move |async_world| async move {
		async_world
			.entity(server)
			.call::<Request, Response>(Request::get("/"))
			.await?;
		Ok(())
	});
	// the boot call never resolves, so settle to its safety cap rather than
	// `flush_async_tasks`, which would wait forever.
	AsyncRunner::settle_async_tasks(&mut world).await;
	world.entity(server).contains::<Listening>().xpect_true();
}

/// Regression (`beet --entry=site serve` bound the http server twice, the second bind
/// failing with "address already in use" and orphaning the live listener's
/// teardown): the site shape is a dispatcher root whose served site hangs off a
/// `serve` route, and a server no longer drags a load verb in with it, so the
/// whole document declares exactly ONE `CallOnReady` and booting it starts each
/// declared server exactly once.
#[beet_core::test]
async fn serving_a_dispatcher_starts_each_server_once() {
	use beet_action::prelude::*;
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	// disarm the load through ancestry, as `serve` does: only the explicit
	// `CallOnReady::call` below may start servers, never the build's own `Ready`.
	let holder = world.spawn(DisableCallOnReady).id();
	let root = spawn_bsx_under(
		&mut world,
		holder,
		"<CliServer always=true {CallOnReady}>\
			<Router>\
				<Route path=\"serve\" {HttpServer}>\
					<Router {RequestLogger}/>\
				</Route>\
			</Router>\
		</CliServer>",
	);
	// the load verb is the root's alone: the servers bring none of their own
	world
		.query_filtered_once::<Entity, With<CallOnReady>>()
		.len()
		.xpect_eq(1);
	// count every start, so a second boot of the serve route is visible. The
	// event sweeps a `SweepDescendants` root's subtree, so count only origins.
	let starts = Store::new(0usize);
	let counter = starts;
	world.add_observer(move |ev: On<StartRunning<Request>>| {
		if ev.trigger().root() == ev.entity {
			counter.set(counter.get() + 1)
		}
	});
	// boot it the way a launched entry does: argv names the `serve` route, on an
	// os-assigned port so a concurrent run never collides
	world.run_async_local(move |async_world| async move {
		CallOnReady::call(
			async_world.entity(root),
			Request::from_cli_str("serve --server=http --port=0"),
		)
		.await
		.ok();
		Ok(())
	});
	// the boot parks (the serve route holds the process), so settle to the cap
	AsyncRunner::settle_async_tasks(&mut world).await;
	let serve_route = world
		.query_filtered_once::<Entity, With<Listening>>()
		.into_iter()
		.next()
		.expect("the serve route's http server never started");
	// bound once: a second bind would have failed rather than listened twice
	world
		.query_filtered_once::<Entity, With<Listening>>()
		.len()
		.xpect_eq(1);
	// the root's own set plus the serve route's, each started exactly once
	starts.get().xpect_eq(2);
	// ..and the teardown reaches that live listener rather than an orphan
	world
		.entity(serve_route)
		.contains::<Running<Response>>()
		.xpect_true();
}

#[beet_core::test]
async fn page_renders_in_layout() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	get(&mut world, root, "docs/intro")
		.await
		.as_str()
		// the layout document wraps the page
		.xpect_contains("<html lang=\"en\">")
		// `RouteHead` lifts the frontmatter title
		.xpect_contains("<title>The Intro</title>")
		// the page body transcludes into the layout's `<main>`
		.xpect_contains("intro body")
		// `RouteSidebar` collects the tree, labelling from frontmatter and
		// marking the active link
		.xpect_contains(">The Intro<")
		.xpect_contains("aria-current=\"page\"");
}

/// A BSX page declares its metadata as `DocMeta` and `RouteMeta` spreads on its root,
/// which the `<RoutesDir/>` scan reads without building the page (so the nav
/// labels it) and the render carries onto the page root (so `RouteHead` titles
/// it) — the same two places markdown frontmatter reaches.
#[beet_core::test]
async fn bsx_page_declares_meta_as_a_spread() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	get(&mut world, root, "docs/spread")
		.await
		.as_str()
		.xpect_contains("<title>The Spread</title>")
		.xpect_contains("spread body")
		.xpect_contains(">The Spread<");
}

/// The counter page through the full route pipeline: every `@` binding value is
/// correct first paint (the default head's `og:site_name` resource bind, the
/// Card's `@prop:title`, the scoped `@doc:count` init), and the page is plain
/// server-rendered html, since the interactivity tier is the wasm binary rather
/// than anything shipped in the markup.
#[beet_core::test]
async fn counter_page_renders_its_bindings() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	let html = get(&mut world, root, "counter").await;
	html.as_str()
		// the default head binds og:site_name to `PackageConfig.title` (no
		// hand-written tag in the layout), so the markup-declared title surfaces
		.xpect_contains("property=\"og:site_name\" content=\"Beet Test Site\"")
		// the Card's `@prop:title` binds the caller's prop into the heading
		.xpect_contains("<h2>Counter</h2>")
		// the scoped `@doc:count=0` init paints its value
		.xpect_contains("You have clicked 0 times.")
		// the `bx:onclick` script is a directive, never an emitted html attribute
		.xnot()
		.xpect_contains("bx:onclick");
	// exactly one og:site_name: the default head owns it, no duplicate.
	html.matches("og:site_name").count().xpect_eq(1);
}

/// Regression: when a host root carries its own command [`RouteTree`] (eg the
/// `beet` CLI's `run-wasm`/`serve` commands from a loaded scene) in the *same*
/// world as a served site, the site's `RouteSidebar` must render only the
/// served site's routes, never leaking the host commands. The sidebar reads its
/// tree off the threaded [`RequestContext::router`] handle — the tree owner
/// resolved once at context build as the nearest tree-bearing ancestor of the
/// matched route — so it renders the served tree, never an arbitrary world
/// `RouteTree`.
///
/// Both route content shapes are covered: a `BlobPage` page (`docs/intro`,
/// whose rendered content is the in-tree route entity) and a per-request page
/// whose rendered content is *detached* from the tree (`page`, the
/// `fixed_func_route` shape the home page uses). The detached case is why the
/// `router` handle is resolved from the matched route entity, not the rendered
/// content (which may sit outside the tree).
#[beet_core::test]
async fn sidebar_excludes_foreign_host_command_tree() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	// a separate host root with its own command route tree, mirroring the dev
	// commands the repo's `main.bsx` wires (run-wasm, export-static, ...).
	world.spawn(children![
		render_action::fixed_func_route(
			"run-wasm",
			|| rsx! { <p>"run-wasm"</p> }
		),
		render_action::fixed_func_route(
			"export-static",
			|| rsx! { <p>"export"</p> }
		),
	]);
	// the served site, a distinct root in the same world, plus a per-request page
	// whose content is built detached (the `fixed_func_route` shape).
	let root = spawn_site(&mut world).await;
	world.spawn((
		ChildOf(root),
		render_action::fixed_func_route("page", || rsx! { <p>"detached"</p> }),
	));
	world.flush();

	// the in-tree `BlobPage` page: its content is the route entity itself.
	let docs = get(&mut world, root, "docs/intro").await;
	docs.as_str().xpect_contains(">The Intro<");
	// the detached per-request page: its content is spawned outside the tree, so
	// only the matched-route anchor resolves the served tree.
	let page = get(&mut world, root, "page").await;
	page.as_str()
		.xpect_contains("detached")
		.xpect_contains(">The Intro<");

	// the host's command routes never leak into either served sidebar.
	for html in [&docs, &page] {
		html.as_str()
			.xnot()
			.xpect_contains("run-wasm")
			.xnot()
			.xpect_contains("export-static");
	}
}

/// An entry instantiating a whole server carries its boot: the document's
/// `CliServer` owns a `<Router/>` dispatch child and declares `{CallOnReady}`,
/// built under the entry root, so the load verb rides the document and the load
/// path finds it. The binary injects no verb of its own.
///
/// The root is built disarmed ([`DisableCallOnReady`]): the assertion is that the
/// machinery *arrived*, and a real boot here would dispatch the test runner's own
/// argv as its request.
#[beet_core::test]
async fn an_instance_carries_boot() {
	let store = BlobStore::temp();
	store
		.insert(
			&RelPath::from("serve.bsx"),
			"<CliServer {CallOnReady}><Router/></CliServer>",
		)
		.await
		.unwrap();
	store
		.insert(
			&RelPath::from("main.bsx"),
			"<Instance document=\"serve.bsx\"/>",
		)
		.await
		.unwrap();
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let entry = store.get_media(&RelPath::from("main.bsx")).await.unwrap();
	let template = BsxParser::body(&world, entry.as_utf8().unwrap()).unwrap();
	// the binary spawns the root with no load verb of its own (`build_root`'s
	// empty `extra`), then builds the entry onto it; the server rides the markup.
	// Disarmed so the assertion is arrival, not a boot with the test argv.
	let root = world.spawn((store, DisableCallOnReady)).id();
	world.entity_mut(root).insert_template(template).unwrap();
	// the document loads as an async pending dependency, so settle first
	AsyncRunner::settle_async_tasks(&mut world).await;
	// the CliServer is the document's root, carrying the load verb, with the
	// Router as its dispatch child.
	let server = world.get::<Activation>(root).unwrap().root;
	world.entity(server).contains::<CliServer>().xpect_true();
	world.entity(server).contains::<CallOnReady>().xpect_true();
	world
		.with_state::<Query<(), With<Router>>, _>(|routers| {
			routers.iter().count()
		})
		.xpect_eq(1);
}

#[beet_core::test]
async fn home_route_serves_index() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	get(&mut world, root, "")
		.await
		.as_str()
		.xpect_contains("welcome")
		.xpect_contains("<title>");
}

// ---- reserved ref names against the real markers ------------------------------

/// Parse a `.bsx` source under `parent`, returning its root element.
fn spawn_bsx_under(world: &mut World, parent: Entity, source: &str) -> Entity {
	let bytes = MediaBytes::new_bsx(source);
	let mut entity = world.spawn(ChildOf(parent));
	ParseContext::new(&mut entity, &bytes).parse().unwrap();
	entity.id()
}

/// The local [`Value`] of the first text child of `entity`.
fn text_value(world: &World, entity: Entity) -> Value {
	let text = world.entity(entity).get::<Children>().unwrap()[0];
	world.entity(text).get::<Value>().unwrap().clone()
}

/// `@entity:PageRoot::` resolves to the nearest render-root ancestor, the
/// in-content replacement for the Rust `RouteHead` meta lookup.
#[beet_core::test]
fn render_root_binding_reads_doc_meta() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	// the route content entity: its own render root, carrying frontmatter meta
	let route = world
		.spawn(DocMeta {
			title: Some("The Title".into()),
			..default()
		})
		.id();
	PageRoot::insert(&mut world.entity_mut(route), default());
	let span = spawn_bsx_under(
		&mut world,
		route,
		"<span>{@entity:PageRoot::DocMeta.title}</span>",
	);
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Str("The Title".into()));

	// reactive: a meta edit reaches the bound text
	world.entity_mut(route).get_mut::<DocMeta>().unwrap().title =
		Some("Renamed".into());
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Str("Renamed".into()));
}

/// A site-wide component on the router entity, bindable from any page.
#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct SiteBrand {
	name: String,
}

/// `@entity:Router::` resolves to the nearest router ancestor, lazily: content
/// built detached stays silent until it attaches beneath the router.
#[beet_core::test]
fn router_binding_resolves_lazily() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	world
		.resource_mut::<AppTypeRegistry>()
		.write()
		.register::<SiteBrand>();
	let router = world
		.spawn((Router, SiteBrand {
			name: "Beet".into(),
		}))
		.id();
	// built detached (the per-request layout pattern): the binding stays silent
	let holder = world.spawn_empty().id();
	let span = spawn_bsx_under(
		&mut world,
		holder,
		"<span>{@entity:Router::SiteBrand.name}</span>",
	);
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Null);

	// attaching beneath the router picks the binding up
	world.entity_mut(holder).insert(ChildOf(router));
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Str("Beet".into()));
}

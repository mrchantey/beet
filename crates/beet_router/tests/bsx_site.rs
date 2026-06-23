//! End-to-end test of the no-code site path (the `bsx_site` example's shape):
//! a `main.bsx` entry declaring the router with middleware spreads, runtime
//! route discovery from a content directory, and a BSX layout composing the
//! route-aware widgets (`RouteHead`, `RouteSidebar`).
beet_core::test_main!();

use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

const MAIN_BSX: &str = r#"
<!-- the whole site: middleware as spreads, routes discovered from disk,
the package resource patched from markup -->
<Router {(RequestLogger, BsxLayout{template:"Layout"})}>
	<PackageConfig title="Beet Test Site" description="markup declared"/>
	<RoutesDir src="routes"/>
	<BlobStoreRoute src="assets"/>
</Router>
"#;

const LAYOUT_BSX: &str = r#"
<html lang="en">
	<RouteHead/>
	<body>
		<RouteSidebar/>
		<main><Slot/></main>
	</body>
</html>
"#;

const CARD_BSX: &str = r#"
<section class="card-filled">
	<h2>{@prop:title}</h2>
	<Slot/>
</section>
"#;

const COUNTER_BSX: &str = r#"
<article bx:scope="counter">
	<widgets::Card title="Counter">
		<p>You have clicked {@doc:count=0} times.</p>
		<button bx:click=increment{ field: @doc:count }>More</button>
	</widgets::Card>
</article>
"#;

/// Write the site fixture (entry, layout template, content routes) and return
/// its root directory.
///
/// Each call gets its own numbered directory: the suite's async tests run
/// concurrently, so a shared fixture dir would let one test's `fs_ext::remove`
/// wipe another's `templates/` mid-scan (a spurious "no template registered").
fn site_fixture() -> AbsPathBuf {
	static SEQ: std::sync::atomic::AtomicU32 =
		std::sync::atomic::AtomicU32::new(0);
	let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let root = AbsPathBuf::new(
		fs_ext::workspace_root().join(format!("target/tests/bsx_site_e2e/{seq}")),
	)
	.unwrap();
	fs_ext::remove(&root).ok();
	fs_ext::write(root.join("main.bsx"), MAIN_BSX).unwrap();
	fs_ext::write(root.join("templates/Layout.bsx"), LAYOUT_BSX).unwrap();
	fs_ext::write(root.join("templates/widgets/Card.bsx"), CARD_BSX).unwrap();
	fs_ext::write(root.join("routes/index.md"), "# Home\n\nwelcome").unwrap();
	fs_ext::write(root.join("routes/counter.bsx"), COUNTER_BSX).unwrap();
	// a static asset the `<BlobStoreRoute src="assets"/>` mount streams
	fs_ext::write(root.join("assets/style.css"), "body { color: red; }")
		.unwrap();
	fs_ext::write(
		root.join("routes/docs/intro.md"),
		"+++\ntitle = \"The Intro\"\norder = 1\n+++\n\n# Intro\n\nintro body",
	)
	.unwrap();
	root
}

/// The example's `main.rs` setup in miniature: plugins + the compile-time
/// package config (the title/description come from `MAIN_BSX`), then register
/// the site templates and spawn the entry.
async fn spawn_site(world: &mut World) -> Entity {
	let site_dir = site_fixture();
	world.insert_resource(pkg_config!());
	// the same store-backed load the binary and Worker run: register `templates/`
	// and read the entry through the site store, then build into a root carrying
	// that store so `RoutesDir` and `<Template src>` resolve it by ancestry.
	let store = BlobStore::new(FsStore::new(site_dir));
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	let sources = read_site_templates(&store, &formats).await.unwrap();
	register_site_templates(world, &formats, sources).unwrap();
	let entry = store.get_media(&SmolPath::from("main.bsx")).await.unwrap();
	let template =
		BsxTemplate::parse_entry(world, entry.as_utf8().unwrap()).unwrap();
	let root = world.spawn(store).id();
	world.entity_mut(root).insert_template(template).unwrap();
	// the `<RoutesDir>` scan is async, so settle it before reading the route tree
	// or serving requests (else the discovered routes 404)
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
	world.entity(root).contains::<BsxLayout>().xpect_true();
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

/// The markup `<BlobStoreRoute src="assets"/>` resolves its `SmolPath` prop and
/// mounts the on-disk `assets/` directory, streaming files beneath it.
#[beet_core::test]
async fn blob_store_route_serves_assets() {
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

/// `<Router {(HttpServer{port:0})}>`: the http server is declarable from markup,
/// landing on the router via the reflect spread path (port 0 keeps any started
/// backend on an OS-assigned port). The reflect insert registers the server's
/// boot (`StartRunning<Request>`) observer through its `on_add`, so the boot fan-out
/// boots it via the installed runtime hook.
// Only without a real HTTP backend: the test installs a stand-in runtime hook to
// prove the wiring without a live server, but with the `http`/`client_io` backend
// present, the boot fan-out boots a real listener (and, under `client_io`, its
// tungstenite channel on a fixed port) that this test cannot cleanly stop, so it
// would leak a spinning task into the rest of the single-process suite.
#[cfg(not(feature = "http"))]
#[beet_core::test]
async fn http_server_declarable_in_markup() {
	// no `server` backend feature here, so install the runtime hook the start
	// observer invokes (idempotent: a prior test may have set it).
	set_http_server(|entity, _shutdown| {
		Box::pin(async move {
			entity
				.with(|mut entity| {
					entity.insert(ServerBooted);
				})
				.await
		})
	})
	.ok();
	// `RouterPlugin` brings in `ServerPlugin`, so it must not be added again.
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let holder = world.spawn_empty().id();
	let root = spawn_bsx_under(
		&mut world,
		holder,
		"<Router {(RequestLogger, HttpServer{port:0})}/>",
	);
	world.entity(root).contains::<Router>().xpect_true();
	world.entity(root).contains::<RequestLogger>().xpect_true();
	world
		.entity(root)
		.get::<HttpServer>()
		.unwrap()
		.port
		.xpect_eq(Some(0));
	// the boot fan-out boots the declared server via the runtime hook: calling
	// the host's `ContinueRun<Boot, Response>` slot reaches the http observer. The
	// call is fire-and-forget: the http boot never resolves (the host parks on its
	// `Running<Response>`), so the stub backend flags the entity then parks.
	world.run_async_local(move |async_world| async move {
		async_world
			.entity(root)
			.call::<Boot, Response>(Boot::from(Request::get("/")))
			.await?;
		Ok(())
	});
	// drive the world so the boot task reaches the stub backend; the boot call
	// itself never resolves (the host parks on `Running<Response>`), so settle to
	// its safety cap rather than `flush_async_tasks`, which would wait forever.
	AsyncRunner::settle_async_tasks(&mut world).await;
	world.entity(root).contains::<ServerBooted>().xpect_true();
}

/// Flag the test runtime hook inserts, proving the boot fan-out reached the
/// declared `HttpServer`'s backend.
#[cfg(not(feature = "http"))]
#[derive(Component)]
struct ServerBooted;

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

/// The counter page through the full route pipeline. The `@` binding values are
/// correct first paint (the default head's `og:site_name` resource bind, the
/// Card's `@prop:title`, the scoped `@doc:count` init); and because the router
/// always renders through the reactive renderer, the same page carries the
/// thin-client wire format: the bound run wrapped in anchors (no flash), the
/// document blob, the event verb with its `@doc` arg resolved absolute, the
/// default verb twins, and the runtime `<script defer>` loaded from the shared
/// `/js/reactivity.js`. The in-browser proof is the Playwright check (Stream 4).
#[beet_core::test]
async fn counter_page_renders_reactively() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	let html = get(&mut world, root, "counter").await;
	html.as_str()
		// --- binding values, correct first paint ---
		// the default head binds og:site_name to `PackageConfig.title` (no
		// hand-written tag in the layout), so the markup-declared title surfaces
		.xpect_contains("property=\"og:site_name\" content=\"Beet Test Site\"")
		// the Card's `@prop:title` binds the caller's prop into the heading
		.xpect_contains("<h2>Counter</h2>")
		// --- reactive wire format ---
		// the document subtree is marked, and the bound run wrapped in anchors with
		// the scoped `@doc:count=0` init between them (correct paint, no overwrite)
		.xpect_contains("data-bx-doc=")
		.xpect_contains(
			"You have clicked <!--bx-ref=\"counter.count\"-->0<!--bx-end--> times.",
		)
		// the event verb re-emitted with its `@doc` arg resolved to an absolute
		// path, so the client needs no scope walk
		.xpect_contains("bx:click=\"increment{ field: @doc:counter.count }\"")
		// the hydration blob, keyed by document id
		.xpect_contains("data-bx-blob")
		.xpect_contains("\"count\":0")
		// the default verb twins ship (the runtime has zero built in)
		.xpect_contains("data-bx-verbs")
		// the runtime loads from the shared cached asset, not an inline script
		.xpect_contains("<script defer src=\"/js/reactivity.js\"></script>");
	// exactly one og:site_name: the default head owns it, no duplicate.
	html.matches("og:site_name").count().xpect_eq(1);
}

/// A page with no `@doc`/`@prop` bindings (the markdown home page) stays
/// byte-clean: the `Auto` reactive renderer emits no blob and no runtime script,
/// so the static output is unchanged.
#[beet_core::test]
async fn plain_page_stays_clean() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world).await;
	get(&mut world, root, "docs/intro")
		.await
		.as_str()
		.xnot()
		.xpect_contains("data-bx")
		.xnot()
		.xpect_contains("/js/reactivity.js");
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
/// Both route content shapes are covered: a `BlobScene` page (`docs/intro`,
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

	// the in-tree `BlobScene` page: its content is the route entity itself.
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

/// Parse a `.bsx` source into a container under `parent`, returning the first
/// content child.
fn spawn_bsx_under(world: &mut World, parent: Entity, source: &str) -> Entity {
	use beet_ui::prelude::*;
	let bytes = MediaBytes::new_bsx(source);
	let mut entity = world.spawn(ChildOf(parent));
	BsxParser::bsx()
		.parse(ParseContext::new(&mut entity, &bytes))
		.unwrap();
	let container = entity.id();
	world.entity(container).get::<Children>().unwrap()[0]
}

/// The local [`Value`] of the first text child of `entity`.
fn text_value(world: &World, entity: Entity) -> Value {
	let text = world.entity(entity).get::<Children>().unwrap()[0];
	world.entity(text).get::<Value>().unwrap().clone()
}

/// `@entity:PageRoot::` resolves to the nearest render-root ancestor, the
/// in-content replacement for the Rust `RouteHead` meta lookup.
#[beet_core::test]
fn render_root_binding_reads_article_meta() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	// the route content entity: its own render root, carrying frontmatter meta
	let route = world
		.spawn(ArticleMeta {
			title: Some("The Title".into()),
			..default()
		})
		.id();
	PageRoot::insert(&mut world.entity_mut(route), default());
	let span = spawn_bsx_under(
		&mut world,
		route,
		"<span>{@entity:PageRoot::ArticleMeta.title}</span>",
	);
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Str("The Title".into()));

	// reactive: a meta edit reaches the bound text
	world
		.entity_mut(route)
		.get_mut::<ArticleMeta>()
		.unwrap()
		.title = Some("Renamed".into());
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

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
</Router>
"#;

const LAYOUT_BSX: &str = r#"
<html lang="en">
	<RouteHead>
		<meta property="og:site_name" content=@res:PackageConfig.title/>
	</RouteHead>
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
		<button bx:click="increment@doc:count">More</button>
	</widgets::Card>
</article>
"#;

/// Write the site fixture (entry, layout template, content routes) and return
/// its root directory.
fn site_fixture() -> AbsPathBuf {
	let root = AbsPathBuf::new(
		fs_ext::workspace_root().join("target/tests/bsx_site_e2e"),
	)
	.unwrap();
	fs_ext::remove(&root).ok();
	fs_ext::write(root.join("main.bsx"), MAIN_BSX).unwrap();
	fs_ext::write(root.join("templates/Layout.bsx"), LAYOUT_BSX).unwrap();
	fs_ext::write(root.join("templates/widgets/Card.bsx"), CARD_BSX).unwrap();
	fs_ext::write(root.join("routes/index.md"), "# Home\n\nwelcome").unwrap();
	fs_ext::write(root.join("routes/counter.bsx"), COUNTER_BSX).unwrap();
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
fn spawn_site(world: &mut World) -> Entity {
	let site_dir = site_fixture();
	world.insert_resource(pkg_config!());
	world.register_bsx_templates(site_dir.join("templates")).unwrap();
	world.insert_resource(SiteRoot(site_dir.clone()));
	BsxTemplate::load_entry(world, site_dir.join("main.bsx"))
		.unwrap()
		.spawn(world)
		.unwrap()
}

/// Request `path`, negotiating HTML, and return the rendered body.
async fn get(world: &mut World, root: Entity, path: &str) -> String {
	world
		.entity_mut(root)
		.call::<Request, Response>(
			Request::get(path)
				.with_header::<header::Accept>(vec![MediaType::Html]),
		)
		.await
		.unwrap()
		.unwrap_str()
		.await
}

#[beet_core::test]
async fn entry_components_land_on_root() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world);
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

/// `<Router {(HttpServer{..}, ..)}>`: the http server is declarable from
/// markup, landing on the router entity via the reflect spread path (port 0
/// keeps any started backend on an OS-assigned port). The `on_add` hook fires
/// through the reflect insert, dispatching the installed no-op backend.
#[beet_core::test]
fn http_server_declarable_in_markup() {
	// no `server` backend feature here, so install the runtime hook the
	// `on_add` dispatch falls through to
	set_http_server(|_| Box::pin(async { Ok(()) })).unwrap();
	let mut world = (AsyncPlugin, RouterPlugin, ServerPlugin).into_world();
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
}

#[beet_core::test]
async fn page_renders_in_layout() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world);
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

/// The `@` bindings against the real route pipeline: a layout `@res` attribute,
/// a `.bsx` template `@prop`, and the counter page's `@doc` text/event bindings,
/// all settled before the SSR render.
#[beet_core::test]
async fn bindings_render_in_route_pipeline() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world);
	get(&mut world, root, "counter")
		.await
		.as_str()
		// the layout head pulls the resource title via `@res:PackageConfig.title`
		.xpect_contains("property=\"og:site_name\" content=\"Beet Test Site\"")
		// the Card's `@prop:title` binds the caller's prop into the heading
		.xpect_contains("<h2>Counter</h2>")
		// the scoped `@doc:count=0` init reaches the text binding
		.xpect_contains("You have clicked 0 times.")
		// the event binding's field mirror stays out of the button label
		.xpect_contains("<button>More</button>");
}

#[beet_core::test]
async fn home_route_serves_index() {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = spawn_site(&mut world);
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

/// `@comp$RenderRoot:` resolves to the nearest render-root ancestor, the
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
	RenderRoot::insert(&mut world.entity_mut(route), default());
	let span = spawn_bsx_under(
		&mut world,
		route,
		"<span>{@comp$RenderRoot:ArticleMeta.title}</span>",
	);
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Str("The Title".into()));

	// reactive: a meta edit reaches the bound text
	world.entity_mut(route).get_mut::<ArticleMeta>().unwrap().title =
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

/// `@comp$Router:` resolves to the nearest router ancestor, lazily: content
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
		"<span>{@comp$Router:SiteBrand.name}</span>",
	);
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Null);

	// attaching beneath the router picks the binding up
	world.entity_mut(holder).insert(ChildOf(router));
	world.update_local();
	text_value(&world, span).xpect_eq(Value::Str("Beet".into()));
}


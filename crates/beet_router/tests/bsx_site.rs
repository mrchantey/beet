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
<!-- the whole site: middleware as spreads, routes discovered from disk -->
<Router {(RequestLogger, BsxLayout{template:"Layout"})}>
	<RoutesDir src="routes"/>
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
	fs_ext::write(root.join("routes/index.md"), "# Home\n\nwelcome").unwrap();
	fs_ext::write(
		root.join("routes/docs/intro.md"),
		"+++\ntitle = \"The Intro\"\norder = 1\n+++\n\n# Intro\n\nintro body",
	)
	.unwrap();
	root
}

/// The example's `main.rs` setup in miniature: plugins + package config, then
/// register the site templates and spawn the entry.
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
	// discovered routes assembled into the root's route tree
	let tree = world.entity(root).get::<RouteTree>().unwrap();
	tree.find(&["docs", "intro"]).is_some().xpect_true();
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

beet::test_main!();

use beet::prelude::*;
use beet_site::prelude::*;

/// Spawn the site router and collect the sidebar nav the layout's
/// [`RouteSidebar`] renders: every route's scan-time [`ArticleMeta`] (emitted
/// by codegen from the markdown frontmatter) drives its label/order/expansion.
fn sidebar_nodes() -> Vec<SidebarNode> {
	let mut world = RouterPlugin.into_world();
	world.insert_resource(pkg_config!());
	let id = world.spawn(beet_site_router()).id();
	world.flush();
	let tree = world.entity(id).get::<RouteTree>().unwrap().clone();
	// no explicit excludes: only `PageRoute`-marked routes reach the nav, so the
	// infra routes (`app-info`/`analytics`/`js/reactivity.js`) drop out on their own.
	let mut state = SidebarState::new("").with_home(false);
	for node in tree.flatten_nodes() {
		if let Some(meta) = world.entity(node.entity).get::<ArticleMeta>() {
			state = state.with_info(node.path.annotated_path(), meta.sidebar_info());
		}
	}
	state.collect(&tree)
}

#[beet::test]
fn excludes_infra_routes() {
	let nodes = sidebar_nodes();
	let names: Vec<&str> = nodes
		.iter()
		.map(|node| node.display_name.as_str())
		.collect();
	// the batteries-included infra routes never appear in the nav (no PageRoute)
	names.iter().any(|name| *name == "app-info").xpect_false();
	names.iter().any(|name| *name == "analytics").xpect_false();
	// the `/js/reactivity.js` asset route is infra too, so its `js` branch is gone
	names.iter().any(|name| *name == "js").xpect_false();
}

#[beet::test]
fn groups_docs_and_blog() {
	let nodes = sidebar_nodes();
	// the index frontmatter pins docs before blog and labels both branches
	nodes[0].display_name.as_str().xpect_eq("Docs");
	nodes[1].display_name.as_str().xpect_eq("Blog");
	// both branches are forced open by their `expanded = true` frontmatter
	nodes[0].expanded.xpect_true();
	nodes[1].expanded.xpect_true();
	// child pages are labelled from their own frontmatter titles
	nodes[0].children.is_empty().xpect_false();
	nodes[1]
		.children
		.iter()
		.any(|node| node.display_name == "Full Stack Bevy")
		.xpect_true();
}

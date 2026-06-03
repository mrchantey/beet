beet::test_main!();

use beet::prelude::*;
use beet_site::prelude::*;

/// Spawn the site router and collect the sidebar nav the shell would render
/// from its [`RouteTree`].
fn sidebar_nodes() -> Vec<SidebarNode> {
	let mut world = RouterPlugin.into_world();
	world.insert_resource(pkg_config!());
	let id = world.spawn(beet_site_router()).id();
	world.flush();
	let tree = world.entity(id).get::<RouteTree>().unwrap().clone();
	SidebarState::new("")
		.with_exclude("app-info")
		.with_exclude("analytics")
		.with_info("docs", SidebarInfo {
			expanded: Some(true),
			..default()
		})
		.with_info("blog", SidebarInfo {
			expanded: Some(true),
			..default()
		})
		.collect(&tree)
}

#[beet::test]
fn excludes_infra_routes() {
	let nodes = sidebar_nodes();
	let names: Vec<&str> = nodes
		.iter()
		.map(|node| node.display_name.as_str())
		.collect();
	// the batteries-included infra routes never appear in the nav
	names.iter().any(|name| *name == "app-info").xpect_false();
	names.iter().any(|name| *name == "analytics").xpect_false();
}

#[beet::test]
fn groups_docs_and_blog() {
	let nodes = sidebar_nodes();
	// a synthetic Home entry plus the docs and blog branches
	nodes[0].display_name.as_str().xpect_eq("Home");
	let docs = nodes
		.iter()
		.find(|node| node.display_name == "docs")
		.unwrap();
	let blog = nodes
		.iter()
		.find(|node| node.display_name == "blog")
		.unwrap();
	// both branches are forced open and carry their child pages
	docs.expanded.xpect_true();
	blog.expanded.xpect_true();
	docs.children.is_empty().xpect_false();
	blog.children
		.iter()
		.any(|node| node.display_name == "post-1")
		.xpect_true();
}

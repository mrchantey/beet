#![cfg_attr(feature = "custom_test_frameworks", allow(unused_features))]
#![cfg_attr(
	feature = "custom_test_frameworks",
	feature(test, custom_test_frameworks)
)]
#![cfg_attr(
	feature = "custom_test_frameworks",
	test_runner(beet::libtest_runner)
)]
use beet::prelude::*;
use beet_site::prelude::*;

// Stable path: the default libtest harness, driven by a single `#[test]` over
// the `inventory`-registered `#[beet::test]` cases.
#[cfg(not(feature = "custom_test_frameworks"))]
#[test]
fn __beet_inventory() { beet::testing::test_main(); }

/// A world with the site's render substrate: the router observers and the
/// Material style rule set, plus the package config the `Head`/`Footer` read.
fn site_world() -> World {
	(
		RouterPlugin,
		material::MaterialStylePlugin::new(palettes::basic::BLUE),
	)
		.into_world()
		.xtap(|world| world.insert_resource(pkg_config!()))
}

/// A `GET {path}` request negotiating HTML (the web render target).
fn html_get(path: &str) -> Request {
	Request::get(path).with_header::<header::Accept>(vec![MediaType::Html])
}

#[beet::test]
async fn home_in_document_shell() {
	site_world()
		.spawn(beet_site_router())
		.exchange_str(html_get(""))
		.await
		// document shell from the layout middleware
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// page body slotted into the shell
		.xpect_contains("A personal application framework")
		// header + sidebar chrome
		.xpect_contains(r#"id="sidebar"#)
		// the content slot is consumed by the layout middleware
		.xnot()
		.xpect_contains(r#"<slot name="main""#);
}

#[beet::test]
async fn docs_renders_sidebar_and_content() {
	site_world()
		.spawn(beet_site_router())
		.exchange_str(html_get("docs"))
		.await
		.xpect_contains(r#"<meta charset="UTF-8""#)
		.xpect_contains(r#"id="sidebar"#)
		// the docs/blog branches and a leaf link are present in the nav
		.xpect_contains("/blog")
		.xpect_contains("/docs");
}

#[beet::test]
async fn blog_post_in_shell() {
	site_world()
		.spawn(beet_site_router())
		.exchange_str(html_get("blog/post-1"))
		.await
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// markdown body rendered inside the shell
		.xpect_contains("Full Stack Bevy");
}

#[beet::test]
async fn terminal_skips_document_shell() {
	// the terminal target negotiates text/ansi, not HTML, so the document shell
	// is skipped and the bare page body is rendered.
	site_world()
		.spawn(beet_site_router())
		.exchange_str(
			Request::get("")
				.with_header::<header::Accept>(vec![MediaType::Text]),
		)
		.await
		// the page body is still present
		.xpect_contains("A personal application framework")
		// but the `<html>`/`<head>` document chrome is not
		.xnot()
		.xpect_contains("<meta charset");
}

#[beet::test]
async fn repeated_requests_are_stable() {
	let mut world = site_world();
	let id = world.spawn(beet_site_router()).id();
	// the shared fixed content must survive request after request: each render
	// must be byte-identical (the layout despawn-hazard regression).
	let first = world.entity_mut(id).exchange_str(html_get("")).await;
	let second = world.entity_mut(id).exchange_str(html_get("")).await;
	first.xpect_eq(second);
}

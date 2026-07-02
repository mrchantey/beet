//! The on-disk `bsx_site` example rendered as a fixture: its real `main.bsx`
//! entry, `templates/`, and `routes/` loaded through a [`BlobStore`] over an
//! [`FsStore`] (the same store-backed build the `beet` binary runs), then served.
//!
//! Where `crates/beet_router/tests/bsx_site.rs` asserts the no-code mechanics from
//! inline `const` BSX, this proves the *committed example files* render: the home
//! and a markdown route inside the layout, the no-code counter's reactive wiring,
//! and a plain page staying byte-clean. Mirrors `examples/rsx_site/tests/render.rs`'s
//! web + terminal assertions, but the site is markup on disk, not Rust.
beet::test_main!();

use beet::prelude::*;

#[path = "bsx_site/mod.rs"]
mod bsx_site;
use bsx_site::build_site;

/// A world with the render substrate the on-disk site loads against, then the built
/// site root: `RouterPlugin` brings the BSX engine + spread server/middleware types
/// (+ `AsyncPlugin`), `MaterialStylePlugin` the style rules the `<Theme>`/`<Rule>`
/// declarations resolve against.
async fn site_world() -> (World, Entity) {
	let mut world =
		(AsyncPlugin, RouterPlugin, material::MaterialStylePlugin).into_world();
	let root = build_site(&mut world).await;
	(world, root)
}

/// A `GET {path}` request negotiating HTML (the web render target).
fn html_get(path: &str) -> Request {
	Request::get(path).with_header::<header::Accept>(vec![MediaType::Html])
}

/// Render `path` against `root`, negotiating HTML.
async fn render(world: &mut World, root: Entity, path: &str) -> String {
	world.entity_mut(root).exchange_str(html_get(path)).await
}

#[beet::test]
async fn entry_lands_on_root() {
	let (world, root) = site_world().await;
	// the spread middleware and servers `main.bsx` declares stack on the router
	world.entity(root).contains::<Router>().xpect_true();
	world.entity(root).contains::<RequestLogger>().xpect_true();
	world.entity(root).contains::<BsxLayout>().xpect_true();
	// the markup `<PackageConfig/>` patched the live resource
	world
		.resource::<PackageConfig>()
		.title
		.as_str()
		.xpect_eq("BSX Site");
	// the on-disk routes assembled into the root's tree
	let tree = world.entity(root).get::<RouteTree>().unwrap();
	tree.find(&["docs", "getting-started"]).xpect_some();
	tree.find(&["blog", "hello-world"]).xpect_some();
	tree.find(&["counter"]).xpect_some();
}

#[beet::test]
async fn home_renders_in_layout() {
	let (mut world, root) = site_world().await;
	render(&mut world, root, "")
		.await
		.as_str()
		// the `templates/Layout.bsx` doctype leads the document, so the served page
		// is a standards-mode HTML document
		.xpect_starts_with("<!DOCTYPE html><html")
		// the `templates/Layout.bsx` `<SiteLayout>` document wraps the page
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// the markdown home body, parsed to elements and transcluded into <main>
		.xpect_contains("A site with no code")
		.xpect_contains("Read the docs")
		// the markup-declared title surfaces through the default head's og:site_name
		.xpect_contains("BSX Site");
}

#[beet::test]
async fn markdown_route_renders_in_layout() {
	let (mut world, root) = site_world().await;
	render(&mut world, root, "docs/getting-started")
		.await
		.as_str()
		// the layout chrome
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// the frontmatter title lifted into the document <title>
		.xpect_contains("<title>Getting Started</title>")
		// the markdown body, parsed to elements
		.xpect_contains("A BSX site is a directory")
		// `RouteSidebar` collects the tree, marking the active link
		.xpect_contains("aria-current=\"page\"");
}

#[beet::test]
async fn blog_markdown_route_renders() {
	let (mut world, root) = site_world().await;
	render(&mut world, root, "blog/hello-world")
		.await
		.as_str()
		.xpect_contains("<title>Hello World</title>")
		.xpect_contains("The obligatory first post");
}

/// The no-code counter page carries the full reactive wire format: the bound run
/// wrapped in anchors with the scoped `@doc:count=0` init, the event verb with its
/// `@doc` arg resolved absolute, the hydration blob, and the shared runtime script.
/// The Rust counter's web mirror, but authored entirely in `routes/counter.bsx`.
#[beet::test]
async fn counter_page_renders_reactively() {
	let (mut world, root) = site_world().await;
	let html = render(&mut world, root, "counter").await;
	html.as_str()
		// the Card template's `@prop:title` binds the caller's prop into the heading
		.xpect_contains("<h2>Counter</h2>")
		// the document subtree marked, the bound run wrapped in anchors with the
		// scoped init between them (correct first paint, no overwrite)
		.xpect_contains("data-bx-doc=")
		.xpect_contains(
			"You have clicked <!--bx-ref=\"counter.count\"-->0<!--bx-end--> times.",
		)
		// the event verb re-emitted with its `@doc` arg resolved to an absolute path
		.xpect_contains("@doc:counter.count")
		// the hydration blob, keyed by document id
		.xpect_contains("data-bx-blob")
		.xpect_contains("\"count\":0")
		// the runtime loads from the shared cached asset, not an inline script
		.xpect_contains("<script defer src=\"/js/reactivity.js\"></script>");
}

/// A page with no `@doc`/`@prop` bindings (a plain markdown doc) stays byte-clean:
/// the `Auto` reactive renderer emits no blob and no runtime script.
#[beet::test]
async fn plain_page_stays_clean() {
	let (mut world, root) = site_world().await;
	render(&mut world, root, "blog/hello-world")
		.await
		.as_str()
		.xnot()
		.xpect_contains("data-bx")
		.xnot()
		.xpect_contains("/js/reactivity.js");
}

#[beet::test]
async fn repeated_requests_are_stable() {
	let (mut world, root) = site_world().await;
	// the shared layout/route content must survive request after request: each
	// render byte-identical (the layout despawn-hazard regression).
	let first = world.entity_mut(root).exchange_str(html_get("")).await;
	let second = world.entity_mut(root).exchange_str(html_get("")).await;
	first.xpect_eq(second);
}

#[beet::test]
async fn terminal_renders_full_layout() {
	let (mut world, root) = site_world().await;
	// the terminal target negotiates text, not HTML, but renders the *full* layout
	// around the body; the non-visual `<head>`/`<style>` simply does not paint.
	world
		.entity_mut(root)
		.exchange_str(
			Request::get("")
				.with_header::<header::Accept>(vec![MediaType::Text]),
		)
		.await
		// the page body is present ...
		.xpect_contains("A site with no code")
		// ... while the non-visual document head never leaks as text
		.xnot()
		.xpect_contains("<meta charset")
		.xnot()
		.xpect_contains("box-sizing");
}

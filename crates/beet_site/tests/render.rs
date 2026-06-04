beet::test_main!();

use beet::prelude::*;
use beet_site::prelude::*;

/// A world with the site's render substrate: the router observers and the
/// Material style rule set, plus the package config the `Head`/`Footer` read.
fn site_world() -> World {
	(
		RouterPlugin,
		material::MaterialStylePlugin::new(palettes::basic::GREEN),
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
		// page body transcluded into the shell's <main>
		.xpect_contains("A personal application framework")
		// header + sidebar chrome
		.xpect_contains(r#"id="sidebar"#);
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
async fn blog_post_title_from_frontmatter() {
	// the per-page `<title>` comes from the post's frontmatter via `ArticleMeta`
	// (queried off the `RequestContext` route entity) -> the shell's `Head`, not
	// the package default.
	site_world()
		.spawn(beet_site_router())
		.exchange_str(html_get("blog/post-1"))
		.await
		.xpect_contains("<title>The Full Moon Harvest #1</title>");
}

#[beet::test]
async fn sidebar_marks_active_route() {
	// the sidebar reads the current path from `RequestContext`, marking the active
	// leaf with `aria-current="page"`.
	site_world()
		.spawn(beet_site_router())
		.exchange_str(html_get("blog/post-1"))
		.await
		.xpect_contains(r#"aria-current="page""#);
}

#[beet::test]
async fn terminal_renders_full_shell() {
	// the terminal target negotiates text, not HTML, but now renders the *full*
	// document shell (header, sidebar, footer) around the body — the non-visual
	// `<head>`/`<style>` simply does not paint, so no markup or CSS leaks.
	site_world()
		.spawn(beet_site_router())
		.exchange_str(
			Request::get("")
				.with_header::<header::Accept>(vec![MediaType::Text]),
		)
		.await
		// the page body is present ...
		.xpect_contains("A personal application framework")
		// ... wrapped in the shell chrome (a header nav link, a sidebar entry) ...
		.xpect_contains("Docs")
		// ... while the non-visual document head never leaks as text
		.xnot()
		.xpect_contains("<meta charset")
		.xnot()
		.xpect_contains("box-sizing");
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

/// Strip ANSI/OSC escape sequences, leaving the visible glyphs.
fn strip_ansi(body: &str) -> String {
	let mut out = String::new();
	let mut chars = body.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch != '\u{1b}' {
			out.push(ch);
			continue;
		}
		match chars.peek() {
			// CSI: ESC [ … final-letter
			Some('[') => {
				for next in chars.by_ref() {
					if next.is_ascii_alphabetic() {
						break;
					}
				}
			}
			// OSC: ESC ] … BEL or ST
			Some(']') => {
				while let Some(next) = chars.next() {
					if next == '\u{7}' {
						break;
					}
					if next == '\u{1b}' {
						chars.next();
						break;
					}
				}
			}
			_ => {}
		}
	}
	out
}

#[beet::test]
async fn terminal_renders_charcell_shell_layout() {
	// the CLI target negotiates AnsiTerm first, driving the charcell layout
	// engine (not the plain-text fallback): the shell becomes a real two-column
	// layout with an elevated header/footer and a sidebar divider.
	let body = site_world()
		.spawn(beet_site_router())
		.exchange_str(
			Request::get("").with_header::<header::Accept>(vec![
				MediaType::AnsiTerm,
				MediaType::Text,
			]),
		)
		.await;

	// the header/footer elevation dividers and the sidebar's right border
	body.as_str()
		.xpect_contains("─") // header/footer horizontal divider
		.xpect_contains("│") // sidebar right border
		// some styling was applied (bold/colour escapes), proving the prose +
		// material rules reach the charcell paint, not raw text
		.xpect_contains("\u{1b}[");

	// sidebar and main share rows: a line holds both a nav entry and body text
	strip_ansi(&body)
		.lines()
		.any(|line| line.contains("blog") && line.contains("Beet"))
		.xpect_true();
}

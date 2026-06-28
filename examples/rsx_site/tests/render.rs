beet::test_main!();

use beet::prelude::*;
use rsx_site::prelude::*;

/// A world with the site's render substrate: the router observers and the
/// Material style rule set, plus the package config the `Header`/`Footer` read.
fn site_world() -> World {
	// `Theme` defaults to the brand green, so the plugin needs no seed colour.
	(RouterPlugin, material::MaterialStylePlugin)
		.into_world()
		.xtap(|world| world.insert_resource(pkg_config!()))
}

/// A `GET {path}` request negotiating HTML (the web render target).
fn html_get(path: &str) -> Request {
	Request::get(path).with_header::<header::Accept>(vec![MediaType::Html])
}

#[beet::test]
async fn home_in_document_layout() {
	site_world()
		.spawn(rsx_site_router())
		.exchange_str(html_get(""))
		.await
		// document layout from the layout middleware
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// page body transcluded into the layout's <main>
		.xpect_contains("A malleable application framework")
		// header chrome: the nav links the layout composes
		.xpect_contains("Counter")
		.xpect_contains("Buttons");
}

#[beet::test]
async fn markdown_content_route_renders_in_layout() {
	// the `content` collection serves `guide.md` through a `BlobScene`, parsed per
	// request, wrapped in the same document layout as the typed pages.
	site_world()
		.spawn(rsx_site_router())
		.exchange_str(html_get("guide"))
		.await
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// the markdown body, parsed to elements
		.xpect_contains("Guide")
		.xpect_contains("markdown")
		// header chrome from the shared layout
		.xpect_contains("Counter");
}

#[beet::test]
async fn dynamic_page_reads_request() {
	// the `about` page is `async fn get(ActionContext<Request>)` (the `async_route`
	// codegen branch), built per request with access to the live request path.
	site_world()
		.spawn(rsx_site_router())
		.exchange_str(html_get("about"))
		.await
		.xpect_contains("About")
		// the per-request body echoes the negotiated path
		.xpect_contains("/about");
}

#[beet::test]
async fn add_server_action_sums_json_body() {
	// the migrated server action: a POST route exchanging a `Json<AddArgs>` body for
	// the sum, dispatched in-process through the same router the pages serve.
	let sum: i32 = site_world()
		.spawn(rsx_site_router())
		.exchange(
			Request::post("add")
				.with_accept(MediaType::Json)
				.with_json_body(&AddArgs { a: 10, b: 20 })
				.unwrap(),
		)
		.await
		.into_result()
		.await
		.unwrap()
		.json()
		.await
		.unwrap();
	sum.xpect_eq(30);
}

#[beet::test]
async fn buttons_page_renders_in_layout() {
	site_world()
		.spawn(rsx_site_router())
		.exchange_str(html_get("buttons"))
		.await
		.xpect_contains(r#"<meta charset="UTF-8""#)
		// the showcase body, laid out by the site-local `design-row` typed Rule
		.xpect_contains("design-row")
		.xpect_contains("Outlined");
}

/// The Rust-authored counter renders reactively on the web: its `{count}`
/// display is wrapped in `<!--bx-ref-->` anchors with the SSR value, the document
/// state ships as a blob, and the runtime loads from the shared
/// `/js/reactivity.js`, so the count hydrates with no flash. Its buttons drive
/// the count through native `PointerUp` observers (the native-target path); the
/// fully web-interactive `bx:click` verb counter is the no-code `bsx_site` page.
#[beet::test]
async fn counter_renders_reactively() {
	site_world()
		.spawn(rsx_site_router())
		.exchange_str(html_get("counter"))
		.await
		// the count display is a bound run wrapping the correct SSR value
		.xpect_contains("<!--bx-ref=")
		.xpect_contains("<!--bx-end-->")
		// the hydration blob carries the initial document state
		.xpect_contains("data-bx-blob")
		.xpect_contains("\"count\":0")
		// the runtime ships, loaded from the shared cached asset
		.xpect_contains("<script defer src=\"/js/reactivity.js\">");
}

#[beet::test]
async fn repeated_requests_are_stable() {
	let mut world = site_world();
	let id = world.spawn(rsx_site_router()).id();
	// the shared fixed content must survive request after request: each render
	// must be byte-identical (the layout despawn-hazard regression).
	let first = world.entity_mut(id).exchange_str(html_get("")).await;
	let second = world.entity_mut(id).exchange_str(html_get("")).await;
	first.xpect_eq(second);
}

#[beet::test]
async fn terminal_renders_full_layout() {
	// the terminal target negotiates text, not HTML, but renders the *full*
	// document layout (header, footer) around the body — the non-visual
	// `<head>`/`<style>` simply does not paint, so no markup or CSS leaks.
	site_world()
		.spawn(rsx_site_router())
		.exchange_str(
			Request::get("")
				.with_header::<header::Accept>(vec![MediaType::Text]),
		)
		.await
		// the page body is present ...
		.xpect_contains("A malleable application framework")
		// ... wrapped in the layout chrome (a header nav link) ...
		.xpect_contains("Counter")
		// ... while the non-visual document head never leaks as text
		.xnot()
		.xpect_contains("<meta charset")
		.xnot()
		.xpect_contains("box-sizing");
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
async fn terminal_renders_charcell_layout() {
	// the CLI target negotiates AnsiTerm first, driving the charcell layout
	// engine (not the plain-text fallback): an elevated header/footer with styling
	// applied, proving the prose + material rules reach the charcell paint.
	let body = site_world()
		.spawn(rsx_site_router())
		.exchange_str(Request::get("").with_header::<header::Accept>(vec![
			MediaType::AnsiTerm,
			MediaType::Text,
		]))
		.await;

	body.as_str()
		.xpect_contains("─") // header/footer horizontal divider
		// some styling was applied (bold/colour escapes), proving the prose +
		// material rules reach the charcell paint, not raw text
		.xpect_contains("\u{1b}[");
	// the body text survives the charcell paint (folded from fullwidth, as the
	// large-font tagline paints in fullwidth glyphs)
	from_fullwidth(&strip_ansi(&body))
		.xpect_contains("malleable application framework");
}

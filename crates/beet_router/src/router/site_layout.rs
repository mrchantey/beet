//! The shipped document-shell layout: a zero-config beet site chrome a no-code
//! BSX author wraps their pages in, with every region an overridable slot.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;
// the document-chrome widgets, imported by name so the tags resolve regardless of
// which feature set's glob (`crate::prelude` vs `beet_ui::prelude`) also defines a
// `Reset`/`Header`.
use beet_ui::prelude::Header;
use beet_ui::prelude::Reset;

/// The shipped document shell wrapping every route's body, slot-driven so a
/// zero-config author writes `<SiteLayout>` and gets the full beet look, while a
/// customizing author fills only the slot they want.
///
/// Reproduces the reference `BeetLayout`: the `<body>` carries [`classes::PAGE`]
/// plus a resolved color-scheme class, and the web-only `<head>` chrome (the
/// [`Stylesheet`] CSS bake, preflight/reset, color-scheme script, favicon) is
/// emitted only for the HTML target. The terminal's `<head>` is `display: none`,
/// so baking the whole rule set to CSS there is pure cost; the gate is a perf
/// guard, never a visual one.
///
/// Color scheme resolution mirrors the reference: an explicit
/// `?color-scheme=light|dark` pins the scheme on both targets; absent it, a
/// non-html target (the terminal) uses the session [`Theme::scheme`] defaulting
/// to [`ColorScheme::Dark`], and the web adds no class (the browser's
/// `color_scheme.js` follows the OS).
///
/// Slots, each defaulting to the standard chrome:
/// - `head`: EXTRA head content appended to the always-emitted chrome inside
///   [`RouteHead`] (eg a live-reload script), so a site adds to the head without
///   losing the chrome.
/// - `header`: defaults to the library [`Header`] with a [`MenuButton`] and the
///   Docs/Blog/GitHub nav links.
/// - `sidebar`: defaults to the route-tree [`RouteSidebar`] (`home=false`).
/// - `footer`: defaults to the library [`Footer`].
/// - the default `<Slot/>` inside `<main>` holds the page body, transcluded by
///   the layout middleware.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so a
/// BSX site declares `<SiteLayout>`. Builds inside a layout render (it reads
/// [`RequestContextStack`]).
#[template(system)]
pub fn SiteLayout(
	stack: Res<RequestContextStack>,
	// the app-wide scheme default a TUI session seeds from `--color-scheme` (see
	// `TuiServer`); the web ignores it and follows the OS.
	theme: Res<Theme>,
) -> impl Bundle {
	let cx = stack.current();
	// an explicit `?color-scheme=light|dark` pins the scheme on both targets via
	// a body class. Absent it, the web follows the OS (`color_scheme.js`); a
	// non-html target (the terminal) uses the session's app-wide scheme,
	// defaulting to dark.
	let mut body_classes = Classes::new([classes::PAGE]);
	match cx
		.parts()
		.get_param("color-scheme")
		.and_then(ColorScheme::parse)
	{
		Some(scheme) => {
			body_classes.insert_class(scheme.class());
		}
		None if !cx.parts().accepts(MediaType::Html) => {
			body_classes.insert_class(theme.scheme.class());
		}
		None => {}
	}
	// The web `<head>` chrome (the `<Stylesheet/>` CSS bake, preflight/reset,
	// color-scheme script) is non-visual in the terminal, where `<head>` is
	// `display: none`. Baking the whole rule set to CSS on every navigation is
	// pure cost there, so only emit it for the HTML target.
	let html_head = cx.parts().accepts(MediaType::Html).then(|| {
		rsx! {
			<Preflight/>
			<Reset/>
			<Stylesheet/>
			<ColorSchemeScript/>
			<link rel="icon" href="/assets/branding/favicon-32x32.png"/>
		}
	});
	rsx! {
		<html lang="en">
			<RouteHead>
				{html_head}
				<Slot name="head"/>
			</RouteHead>
			<body {body_classes}>
				<Slot name="header">
					<Header>
						<MenuButton slot="leading"/>
						<Link slot="nav" href="/docs" variant=ButtonVariant::Text>"Docs"</Link>
						<Link slot="nav" href="/blog" variant=ButtonVariant::Text>"Blog"</Link>
						<Link slot="nav" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text>"GitHub"</Link>
					</Header>
				</Slot>
				<div {Classes::new([classes::CONTAINER])}>
					<Slot name="sidebar">
						<RouteSidebar home=false/>
					</Slot>
					<main>
						<Slot/>
					</main>
				</div>
				<Slot name="footer">
					<Footer/>
				</Slot>
			</body>
		</html>
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// A router world (which installs the [`RequestContextStack`] and registers the
	/// layout widgets) seeded with the request-scoped facts a layout reads: a
	/// one-route tree the default [`RouteSidebar`] collects against, doubling as the
	/// context's content/route/router anchor.
	fn layout_world(parts: RequestParts) -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// the `Header`/`RouteHead` chrome reads the site name off `PackageConfig`;
		// the live middleware seeds it, so a bare render world must too.
		world.init_resource::<PackageConfig>();
		// the layout reads the app-wide scheme default off `Theme`, seeded here as a
		// bare render world omits `MaterialStylePlugin`.
		world.init_resource::<Theme>();
		let route = world
			.spawn((
				render_action::fixed_func_route("", || rsx! { <p>"body"</p> }),
				PageRoute,
			))
			.flush();
		world
			.resource_mut::<RequestContextStack>()
			.push(RequestContext::new(parts, route, route, route));
		world
	}

	/// A `GET` request pinning the given `Accept` media type, so the layout's
	/// HTML-gate resolves deterministically (an unset `Accept` accepts everything).
	fn request(query: &str, accept: MediaType) -> RequestParts {
		let mut parts = RequestParts::get(query);
		parts.headers_mut().set::<header::Accept>(vec![accept]);
		parts
	}

	/// Render `<SiteLayout>` (with a page-body slot child) to HTML for the given
	/// request parts.
	fn render(parts: RequestParts) -> String {
		let mut world = layout_world(parts);
		let entity = world
			.spawn_template(rsx! {
				<SiteLayout>
					<p>"page body"</p>
				</SiteLayout>
			})
			.unwrap()
			.id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	/// A pinned `?color-scheme=light` request paints `page` plus the light-scheme
	/// class on the body.
	#[beet_core::test]
	fn pinned_scheme_on_body() {
		render(request("?color-scheme=light", MediaType::Html))
			.xpect_contains(classes::PAGE.as_selector().as_str())
			.xpect_contains(classes::LIGHT_SCHEME.as_selector().as_str());
	}

	/// The web-only chrome (`<Stylesheet/>` bakes a `<style>`) is emitted for an
	/// HTML request.
	#[beet_core::test]
	fn chrome_present_for_html() {
		render(request("", MediaType::Html)).xpect_contains("<style");
	}

	/// The same chrome is gated off for a non-html (terminal) request: baking the
	/// rule set to CSS there is wasted, since the terminal's `<head>` is hidden.
	#[beet_core::test]
	fn chrome_absent_for_non_html() {
		render(request("", MediaType::AnsiTerm))
			.xpect_contains("page")
			.xnot()
			.xpect_contains("<style");
	}
}

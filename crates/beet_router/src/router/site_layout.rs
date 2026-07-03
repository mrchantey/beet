//! The shipped document-shell layout: a zero-config beet site chrome a no-code
//! BSX author wraps their pages in, with every region an overridable slot.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::RequestParts;
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
	// `TuiServer`); the web ignores it and follows the OS. `RouterPlugin` inits
	// `Theme`, so this resolves even without `MaterialStylePlugin`.
	theme: Res<Theme>,
) -> impl Bundle {
	let cx = stack.current();
	// `PAGE` plus the resolved scheme class (an explicit `?color-scheme=` on both
	// targets, else the session default on the terminal, else none on the web so
	// `color_scheme.js` follows the OS); shared with the help/not-found page.
	let body_classes = page_classes(cx.parts(), &theme);
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
		<!DOCTYPE html>
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

/// The page body classes for a request: [`classes::PAGE`] plus the resolved
/// color-scheme class.
///
/// An explicit `?color-scheme=light|dark` pins the scheme on both targets; absent
/// it, a non-html target (the terminal) uses the session [`Theme::scheme`]
/// (defaulting to dark), and the web adds no class so its `color_scheme.js`
/// follows the OS. Shared by [`SiteLayout`] and the help/not-found page so a
/// bare-rendered page (eg the dev CLI help, with no layout) is themed the same.
pub(crate) fn page_classes(parts: &RequestParts, theme: &Theme) -> Classes {
	let mut classes = Classes::new([classes::PAGE]);
	let scheme =
		match parts.get_param("color-scheme").and_then(ColorScheme::parse) {
			Some(scheme) => Some(scheme),
			None if !parts.accepts(MediaType::Html) => Some(theme.scheme),
			None => None,
		};
	if let Some(scheme) = scheme {
		classes.insert_class(scheme.class());
	}
	classes
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
	/// context's content/route/router anchor. Includes the Material rule set so
	/// `<Stylesheet/>` bakes the same rules a deployed site serves.
	fn layout_world(parts: RequestParts) -> World {
		let mut world =
			(AsyncPlugin, RouterPlugin, material::MaterialStylePlugin::default())
				.into_world();
		// the `Header`/`RouteHead` chrome reads the site name off `PackageConfig`;
		// the live middleware seeds it, so a bare render world must too.
		world.init_resource::<PackageConfig>();
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

	/// A wildcard `Accept` (`*/*`, the curl/bot default) gets the same styled
	/// chrome: it renders the html page, so it must ship the html head.
	/// Regression: the bare variant was cacheable html, so one wildcard request
	/// poisoned the edge cache with an unstyled page for every browser.
	#[beet_core::test]
	fn chrome_present_for_wildcard() {
		render(request("", MediaType::from_content_type("*/*")))
			.xpect_contains("<style");
	}

	/// No-flash sidebar: the served `<nav id="sidebar">` carries no `aria-hidden`
	/// attribute, and the baked stylesheet hides the rail below the breakpoint
	/// unless `sidebar.js` has set `aria-hidden="false"` - so a narrow-screen
	/// first paint never shows the rail while the script defers.
	#[beet_core::test]
	fn sidebar_hidden_before_script() {
		let html = render(request("", MediaType::Html));
		// the nav's open tag ships without an aria-hidden attribute
		let idx = html.find("id=\"sidebar\"").unwrap();
		let open = html[..idx].rfind('<').unwrap();
		let close = idx + html[idx..].find('>').unwrap();
		html[open..close].xnot().xpect_contains("aria-hidden");
		// the stylesheet's CSS-first collapse rule is baked into the page
		html.as_str()
			.xpect_contains(&format!(
				"(max-width: {}px)",
				classes::SIDEBAR_BREAKPOINT_PX
			))
			.xpect_contains(":not([aria-hidden=");
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

	/// [`page_classes`] resolution: a terminal request defaults to the session
	/// scheme (dark), the web adds no scheme class (it follows the OS), and an
	/// explicit `?color-scheme=` pins the scheme on either target.
	#[beet_core::test]
	fn page_classes_resolves_scheme() {
		let theme = Theme::default();
		let has =
			|classes: &Classes, name: &ClassName| classes.contains_name(name);
		// a terminal request with no override → PAGE + the dark session default
		let terminal = page_classes(&request("", MediaType::AnsiTerm), &theme);
		has(&terminal, &classes::PAGE).xpect_true();
		has(&terminal, &classes::DARK_SCHEME).xpect_true();
		// the web adds no scheme class, leaving the OS to drive `color_scheme.js`
		let web = page_classes(&request("", MediaType::Html), &theme);
		has(&web, &classes::PAGE).xpect_true();
		has(&web, &classes::DARK_SCHEME).xpect_false();
		has(&web, &classes::LIGHT_SCHEME).xpect_false();
		// an explicit pin wins on both targets
		let pinned = page_classes(
			&request("?color-scheme=light", MediaType::Html),
			&theme,
		);
		has(&pinned, &classes::LIGHT_SCHEME).xpect_true();
	}
}

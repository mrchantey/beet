use crate::prelude::*;
use beet::prelude::*;

/// The global document layout wrapping every route's body.
///
/// Composes the library [`Header`]/[`Footer`] and the route-tree
/// [`RouteSidebar`] (labels/order/expansion sourced from each route's
/// scan-time [`ArticleMeta`]) around the route content (the default `<Slot/>`,
/// transcluded in place by the [`BaseLayout`] middleware). The shared [`RouteHead`] carries the web-only
/// stylesheet/color-scheme/preflight/favicon, sourcing the title/description
/// from the matched route's [`ArticleMeta`]. The `<head>` is non-visual, so the
/// same layout renders in the terminal.
#[template(system)]
pub fn BeetLayout(
	cx: Res<RequestContext>,
	// the app-wide scheme a TUI session seeds from `--color-scheme` (see
	// `TuiServer`); absent on the web.
	app_scheme: Option<Res<AppColorScheme>>,
) -> impl Bundle {
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
			let scheme =
				app_scheme.map(|scheme| **scheme).unwrap_or(ColorScheme::Dark);
			body_classes.insert_class(scheme.class());
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
			</RouteHead>
			<body {body_classes}>
				<Header>
					<MenuButton slot="leading"/>
					<Link slot="nav" href=routes::docs::index() variant=ButtonVariant::Text>"Docs"</Link>
					<Link slot="nav" href=routes::blog::index() variant=ButtonVariant::Text>"Blog"</Link>
					<Link slot="nav" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text>"GitHub"</Link>
				</Header>
				<div {Classes::new([classes::CONTAINER])}>
					<RouteSidebar home=false/>
					<main>
						<Slot/>
					</main>
				</div>
				<Footer/>
			</body>
		</html>
	}
}

use crate::prelude::*;
use beet::prelude::*;

/// The global document layout wrapping every route's body.
///
/// Composes the library [`Header`]/[`Footer`] around the route content (the
/// default `<Slot/>`, transcluded in place by the [`BaseLayout`] middleware). The
/// shared [`RouteHead`] carries the web-only stylesheet/color-scheme/preflight,
/// sourcing the title/description from the matched route's [`ArticleMeta`]. The
/// `<head>` is non-visual, so the same layout renders in the terminal.
#[template(system)]
pub fn BeetLayout(
	stack: Res<RequestContextStack>,
	// the app-wide scheme a TUI session seeds from `--color-scheme` (see
	// `TuiServer`); absent on the web.
	app_scheme: Option<Res<AppColorScheme>>,
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
		}
	});
	rsx! {
		<html lang="en">
			<RouteHead>
				{html_head}
			</RouteHead>
			<body {body_classes}>
				<Header>
					<Link slot="nav" href=routes::counter() variant=ButtonVariant::Text>"Counter"</Link>
					<Link slot="nav" href=routes::buttons() variant=ButtonVariant::Text>"Buttons"</Link>
					<Link slot="nav" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text>"GitHub"</Link>
				</Header>
				<div {Classes::new([classes::CONTAINER])}>
					<main>
						<Slot/>
					</main>
				</div>
				<Footer/>
			</body>
		</html>
	}
}

use crate::prelude::*;
use beet::prelude::*;

/// The global document shell wrapping every route's body.
///
/// Composes the library [`Header`]/[`Footer`] and the site [`BeetSidebar`]
/// around the route content (the default `<slot/>`, transcluded in place by the
/// [`DocumentShell`] middleware). The library [`Head`] carries the web-only
/// stylesheet/color-scheme/preflight/favicon, with its title/description sourced
/// from the matched route's [`ArticleMeta`] (markdown frontmatter, queried off
/// the [`RequestContext`] route entity, falling back to [`PackageConfig`]). The
/// `<head>` is non-visual, so the same shell renders in the terminal.
#[scene(system)]
pub fn BeetDocumentShell(
	cx: Res<RequestContext>,
	metas: Query<&ArticleMeta>,
	pkg: Res<PackageConfig>,
) -> impl Scene {
	let meta = metas.get(cx.route()).ok();
	let title = meta
		.and_then(|meta| meta.title.clone())
		.unwrap_or_else(|| pkg.title.clone());
	let description = meta
		.and_then(|meta| meta.description.clone())
		.unwrap_or_else(|| pkg.description.clone());
	// an explicit `?color-scheme=light|dark` pins the scheme on both targets via
	// a body class. Absent it, the web follows the OS (`color_scheme.js`); a
	// non-html target (the terminal) defaults to dark.
	let mut body_classes = Classes::new([classes::PAGE]);
	match cx.parts().get_param("color-scheme") {
		Some("light") => {
			body_classes.insert_class(classes::LIGHT_SCHEME);
		}
		Some("dark") => {
			body_classes.insert_class(classes::DARK_SCHEME);
		}
		_ if !cx.parts().accepts(MediaType::Html) => {
			body_classes.insert_class(classes::DARK_SCHEME);
		}
		_ => {}
	}
	rsx! {
		<html lang="en">
			<Head title=title description=description>
				<Preflight/>
				<Reset/>
				<Stylesheet/>
				<ColorSchemeScript/>
				<link rel="icon" href="/assets/branding/favicon-32x32.png"/>
			</Head>
			<body {body_classes}>
				<Header>
					<Link slot="nav" href=routes::docs::index() variant=ButtonVariant::Text>"Docs"</Link>
					<Link slot="nav" href=routes::blog::index() variant=ButtonVariant::Text>"Blog"</Link>
					<Link slot="nav" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text>"GitHub"</Link>
				</Header>
				<div {Classes::new([classes::CONTAINER])}>
					<BeetSidebar/>
					<main>
						<slot/>
					</main>
				</div>
				<Footer/>
			</body>
		</html>
	}
}

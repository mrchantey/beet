use crate::prelude::*;
use beet::prelude::*;

/// The global document shell wrapping every route's body.
///
/// Composes the library [`Header`]/[`Footer`] and the site [`BeetSidebar`]
/// around the route content (the default `<slot/>`, transcluded in place by the
/// [`DocumentShell`] middleware). The library [`Head`] carries the web-only
/// stylesheet/color-scheme/preflight/favicon, with its title/description sourced
/// from the matched route's [`RouteContext`] (markdown frontmatter, falling back
/// to [`PackageConfig`]). The `<head>` is non-visual, so the same shell renders
/// in the terminal.
#[scene(system)]
pub fn BeetDocumentShell(
	cx: &RouteContext,
	pkg: Res<PackageConfig>,
) -> impl Scene {
	let meta = cx.article_meta();
	let title = meta.title.clone().unwrap_or_else(|| pkg.title.clone());
	let description =
		meta.description.clone().unwrap_or_else(|| pkg.description.clone());
	rsx! {
		<html lang="en">
			<Head title=title description=description>
				<Preflight/>
				<Stylesheet/>
				<ColorSchemeScript/>
				<link rel="icon" href="/assets/branding/favicon-32x32.png"/>
			</Head>
			<body>
				<Header>
					<Link slot="nav" label="Docs" href=routes::docs::index() variant=ButtonVariant::Text/>
					<Link slot="nav" label="Blog" href=routes::blog::index() variant=ButtonVariant::Text/>
					<Link slot="nav" label="GitHub" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text/>
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

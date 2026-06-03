use crate::prelude::*;
use beet::prelude::*;

/// The global document shell wrapping every route's body.
///
/// Composes the library [`Header`]/[`Footer`] and the site [`BeetSidebar`]
/// around the route content (the default `<slot/>`, transcluded in place by the
/// [`document_shell`] middleware), with a context-aware [`BeetHead`] carrying
/// the web-only stylesheet/color-scheme/preflight/favicon. The `<head>` is
/// non-visual, so the same shell renders in the terminal.
#[scene]
pub fn BeetDocumentShell() -> impl Scene {
	rsx! {
		<html lang="en">
			<BeetHead>
				<Preflight/>
				<Stylesheet/>
				<ColorSchemeScript/>
				<link rel="icon" href="/assets/branding/favicon-32x32.png"/>
			</BeetHead>
			<body>
				<Header>
					<Link slot="nav" label="Docs" href=routes::docs::index() variant=ButtonVariant::Text/>
					<Link slot="nav" label="Blog" href=routes::blog::index() variant=ButtonVariant::Text/>
					<Link slot="nav" label="GitHub" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text/>
				</Header>
				<div {Classes::new([classes::CONTAINER])}>
					<BeetSidebar/>
					<main {Classes::new(["site-main"])}>
						<slot/>
					</main>
				</div>
				<Footer/>
			</body>
		</html>
	}
}

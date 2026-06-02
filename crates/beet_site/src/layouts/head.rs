//! Site `<head>`: a context-aware wrapper over the library [`Head`].
use beet::prelude::*;

/// The site `<head>`.
///
/// Forwards the matched route's title/description (from the [`RouteContext`],
/// sourced from markdown frontmatter, falling back to [`PackageConfig`]) to the
/// library [`Head`], and appends the web-only stylesheet/color-scheme/preflight/
/// favicon supplied as `children`. The library `Head` is non-visual, so this is
/// inert in the terminal.
#[scene(system)]
pub fn BeetHead(
	cx: &RouteContext,
	pkg: Res<PackageConfig>,
	#[prop] children: SceneProp,
) -> impl Scene {
	let meta = cx.article_meta();
	let title = meta.title.clone().unwrap_or_else(|| pkg.title.clone());
	let description =
		meta.description.clone().unwrap_or_else(|| pkg.description.clone());
	rsx! {
		<Head title=title description=description>
			{children}
		</Head>
	}
}

//! Page `<header>` widget — app bar with a title link and a `<nav>` slot.
use beet_core::prelude::*;

/// A page `<header>` with a title link to `home_route` (defaults to `/`) and
/// a `<nav>` slot for navigation links.
#[scene(system)]
pub fn Header(
	#[prop(into)] home_route: String,
	pkg_config: Res<PackageConfig>,
) -> impl Scene {
	let title = pkg_config.title.clone();
	let home_route =
		if home_route.is_empty() { "/".to_string() } else { home_route };
	rsx! {
		<header {Classes::new([classes::APP_BAR, classes::PRINT_HIDDEN])}>
			<a {Classes::new(["app-bar-title"])} href={home_route}>
				{title}
			</a>
			<slot/>
			<nav {Classes::new([classes::APP_BAR_NAV])}>
				<slot name="nav"/>
			</nav>
		</header>
	}
}

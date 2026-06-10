//! Page `<header>` widget — app bar with a title link and a `<nav>` slot.
use crate::prelude::*;
use beet_core::prelude::*;

/// A page `<header>` with a title link to `home_route` (defaults to `/`) and
/// a `<nav>` slot for navigation links.
#[template(system)]
pub fn Header(
	#[prop(into)] home_route: String,
	pkg_config: Res<PackageConfig>,
) -> impl Bundle {
	let title = pkg_config.title.clone();
	let home_route = if home_route.is_empty() {
		"/".to_string()
	} else {
		home_route
	};
	rsx! {
		<header {Classes::new([classes::APP_BAR, classes::PRINT_HIDDEN])}>
			<a {Classes::new(["app-bar-title"])} href={home_route}>
				{title}
			</a>
			<Slot/>
			<nav {Classes::new([classes::APP_BAR_NAV])}>
				<Slot name="nav"/>
			</nav>
		</header>
	}
}

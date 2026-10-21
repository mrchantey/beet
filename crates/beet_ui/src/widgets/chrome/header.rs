//! Page `<header>` widget — app bar with a title link and a `<nav>` slot.
use crate::prelude::*;
use beet_core::prelude::*;

/// A page `<header>` with a title link to `home_route` (defaults to `/`) and
/// a `<nav>` slot for navigation links.
///
/// The title is always the site name ([`PackageConfig::title`]), bound through a
/// `@res:PackageConfig.title` [`ResourceFieldRef`] rather than snapshotted, so it
/// stays live with the resource and never picks up a per-route title.
///
/// The `leading` slot holds an optional control left of the title (eg a
/// [`MenuButton`](crate::prelude::MenuButton)); the default slot sits between
/// the leading cluster and the `nav`.
#[template(system)]
pub fn Header(
	#[prop] home_route: String,
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
			<div {Classes::new([classes::APP_BAR_LEADING])}>
				<Slot name="leading"/>
				<a {Classes::new(["app-bar-title"])} href={home_route}>
					{site_title(&title)}
				</a>
			</div>
			<Slot/>
			<nav {Classes::new([classes::APP_BAR_NAV])}>
				<Slot name="nav"/>
			</nav>
		</header>
	}
}

/// The bound text child of the header title link: a [`Value`] seeded with
/// [`PackageConfig::title`] (so SSR renders it before any sync) plus, under
/// `json`, a [`ResourceFieldRef`] keeping it live with the resource. This is the
/// Rust counterpart of `{@res:PackageConfig.title}`; without `json` it stays the
/// static seed.
fn site_title(title: &SmolStr) -> impl Bundle {
	let value = Value::new(title);
	#[cfg(feature = "json")]
	return (value, ResourceFieldRef::new("PackageConfig", "title"));
	#[cfg(not(feature = "json"))]
	return value;
}

/// A [`Header`] `nav` link to the package's source repository
/// ([`PackageConfig::repository`]), labelled by its forge, ie `GitHub`, else
/// `Source`. Renders nothing when the package names no repository, so a shell
/// places it unconditionally.
#[template(system)]
pub fn RepositoryLink(pkg_config: Res<PackageConfig>) -> impl Bundle {
	let link = pkg_config.repository.as_ref().map(|repository| {
		let label = forge_label(repository);
		rsx! {
			<Link href={repository.to_string()} variant=ButtonVariant::Text>{label}</Link>
		}
	});
	rsx! { {link} }
}

/// The name a repository link shows: its forge's, else `Source`.
fn forge_label(repository: &Url) -> &'static str {
	match repository
		.host()
		.map(|host| host.trim_start_matches("www."))
	{
		Some("github.com") => "GitHub",
		Some("codeberg.org") => "Codeberg",
		Some("gitlab.com") => "GitLab",
		Some("tangled.org" | "tangled.sh") => "Tangled",
		_ => "Source",
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Render `<RepositoryLink/>` against a package naming `repository`.
	fn render(repository: Option<&str>) -> String {
		let mut world = world_ext::ui_world();
		world.insert_resource(PackageConfig {
			repository: repository.map(Url::coerce),
			..default()
		});
		let root = world
			.spawn_template(rsx! { <RepositoryLink/> })
			.unwrap()
			.id();
		crate::widgets::test_ext::render_world(&mut world, root)
	}

	#[beet_core::test]
	fn links_the_repository_by_its_forge() {
		render(Some("https://github.com/mrchantey/beet"))
			.xpect_contains("href=\"https://github.com/mrchantey/beet\"")
			.xpect_contains("GitHub");
		render(Some("https://git.example.com/beet")).xpect_contains("Source");
		render(None).xnot().xpect_contains("<a");
	}
}

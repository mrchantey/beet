//! Document-shell widgets: `Head`/`Header`/`Footer` and the `*Layout` widgets
//! that compose them into a full HTML page.
//!
//! Web `<head>` only — non-web targets ignore the produced meta tags during
//! rendering. The meta values are sourced from [`PackageConfig`] at scene build
//! time via `#[scene(system)]`, so the same widget composition fills correctly
//! in any binary that initializes the resource (via `pkg_config!()`).
//!
//! `<head>` is split into `Head` (charset/title/viewport/canonical/description/
//! version/og + a slot for caller overrides) and `head_meta!` (a tag list for
//! callers that want PWA/Apple/Twitter cards; pasted into the slot). The split
//! exists because `bevy_scene`'s `SceneList` tuple impls top out at 12 children
//! per entity, and a full social/PWA meta block exceeds that.
use beet_core::prelude::*;

/// A `<head>` with sensible defaults sourced from [`PackageConfig`].
///
/// Renders charset, title, canonical, viewport (toggle `fixed_scale` for games),
/// description, version, theme-color, application-name, and the core Open Graph
/// tags. Additional meta tags (Apple PWA, Twitter cards, …) should be added
/// through the default slot.
#[scene(system)]
pub fn Head(
	#[prop] fixed_scale: bool,
	pkg_config: Res<PackageConfig>,
) -> impl Scene {
	let PackageConfig {
		title,
		description,
		homepage,
		..
	} = pkg_config.clone();

	let scale = if fixed_scale {
		"width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
	} else {
		"width=device-width, initial-scale=1"
	};

	rsx! {
		<head>
			<meta charset="UTF-8"/>
			<title>{title.clone()}</title>
			<link rel="canonical" href={homepage.clone()}/>
			<meta name="viewport" content={scale}/>
			<meta name="description" content={description.clone()}/>
			<meta name="application-name" content={title.clone()}/>
			<meta name="theme-color" content="#ffffff"/>
			<meta property="og:title" content={title.clone()}/>
			<meta property="og:type" content="website"/>
			<meta property="og:description" content={description.clone()}/>
			<meta property="og:url" content={homepage.clone()}/>
			<slot/>
		</head>
	}
}

/// A page `<header>` with a title link to `home_route` (defaults to `/`) and
/// a `<nav>` slot for navigation links.
#[scene(system)]
pub fn Header(
	#[prop(into)] home_route: String,
	pkg_config: Res<PackageConfig>,
) -> impl Scene {
	let title = pkg_config.title.clone();
	let home_route = if home_route.is_empty() {
		"/".to_string()
	} else {
		home_route
	};
	rsx! {
		<header {Classes::new(["app-bar", "print-hidden"])}>
			<a {Classes::new(["app-bar-title"])} href={home_route}>
				{title}
			</a>
			<slot/>
			<nav>
				<slot name="nav"/>
			</nav>
		</header>
	}
}

/// A page `<footer>` displaying the copyright + version + build stage from
/// [`PackageConfig`].
#[scene(system)]
pub fn Footer(pkg_config: Res<PackageConfig>) -> impl Scene {
	let PackageConfig { title, version, stage, .. } = pkg_config.clone();

	let current_year = current_year();
	let footer_text = format!("© {title} {current_year}");

	let mut build_text = format!("v{version}");
	#[cfg(debug_assertions)]
	build_text.push_str(" | build=debug");
	if stage != "prod" {
		build_text.push_str(&format!(" | stage={stage}"));
	}

	rsx! {
		<footer id="page-footer" {Classes::new(["print-hidden"])}>
			<span>{footer_text}</span>
			<slot/>
			<span>{build_text}</span>
		</footer>
	}
}

/// `chrono` is std-only and not in the `scene` feature graph; the footer just
/// needs the year, so we derive it directly via `std::time` to avoid a new dep.
/// Approximation is fine for a footer string (off by at most a day around new
/// year).
fn current_year() -> i32 {
	use std::time::SystemTime;
	use std::time::UNIX_EPOCH;
	let secs = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs() as i64)
		.unwrap_or(0);
	1970 + (secs as f64 / (365.2425 * 86400.0)) as i32
}

/// Wraps an entire page, including `<head>` and `<body>`.
///
/// Slots: `head` (extra `<head>` content), default (page `<body>`).
#[scene]
pub fn DocumentLayout() -> impl Scene {
	rsx! {
		<html lang="en">
			<Head>
				<slot name="head"/>
			</Head>
			<body>
				<slot/>
			</body>
		</html>
	}
}

/// A standard HTML page: a [`DocumentLayout`] with a [`Header`] and [`Footer`]
/// around a body slot.
///
/// Slots: `head`, `header`, `header-nav`, `footer`, default (page body).
#[scene]
pub fn PageLayout() -> impl Scene {
	rsx! {
		<DocumentLayout>
			<slot name="head" slot="head"/>
			<div {Classes::new(["page"])}>
				<Header>
					<slot name="header"/>
					<slot name="header-nav" slot="nav"/>
				</Header>
				<slot/>
				<Footer>
					<slot name="footer"/>
				</Footer>
			</div>
		</DocumentLayout>
	}
}

/// A [`PageLayout`] with a `<main>` content area for article-style pages.
///
/// Slots: `head`, `header`, `header-nav`, `footer`, default (main content).
#[scene]
pub fn ContentLayout() -> impl Scene {
	rsx! {
		<PageLayout>
			<slot name="head" slot="head"/>
			<slot name="header" slot="header"/>
			<slot name="header-nav" slot="header-nav"/>
			<slot name="footer" slot="footer"/>
			<main {Classes::new(["content-main"])}>
				<slot/>
			</main>
		</PageLayout>
	}
}

//! `<head>` widget — sensible default meta tags sourced from [`PackageConfig`].
//!
//! Web `<head>` only — non-web targets ignore the produced meta tags during
//! rendering. The meta values are sourced from [`PackageConfig`] at scene build
//! time via `#[scene(system)]`, so the same widget composition fills correctly
//! in any binary that initializes the resource (via `pkg_config!()`).
//!
//! `<head>` is split from `<header>`/`<footer>` because it targets the document
//! root, not the page body. The full social/PWA meta block (Open Graph,
//! Twitter cards, Apple/Android PWA, Microsoft tiles) is baked in; the scene
//! `rsx!` lowering chunks >12 children into nested tuples, so the old
//! `SceneList` 12-tuple cap no longer forces them out to the caller. Extra,
//! app-specific tags still flow in through the default slot.
//!
//! Each meta value is converted to a [`SmolStr`] up front so reusing it across
//! tags is a cheap Arc clone; the rsx! macro consumes each interpolation, but
//! the underlying string buffer is shared.
use beet_core::prelude::*;

/// A `<head>` with sensible defaults sourced from [`PackageConfig`].
///
/// Renders charset, title, canonical, viewport (toggle `fixed_scale` for games),
/// description, version, theme-color, application-name, the core Open Graph and
/// Twitter-card tags, and the Apple/Android/Microsoft PWA meta block. Extra
/// app-specific tags can be added through the default slot.
#[scene(system)]
pub fn Head(
	#[prop] fixed_scale: bool,
	pkg_config: Res<PackageConfig>,
) -> impl Scene {
	let title = SmolStr::new(&pkg_config.title);
	let description = SmolStr::new(&pkg_config.description);
	let homepage = SmolStr::new(&pkg_config.homepage);
	let version = SmolStr::new(&pkg_config.version);

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
			<meta name="version" content={version}/>
			<meta name="application-name" content={title.clone()}/>
			<meta name="theme-color" content="#ffffff"/>
			// Open Graph
			<meta property="og:title" content={title.clone()}/>
			<meta property="og:type" content="website"/>
			<meta property="og:description" content={description.clone()}/>
			<meta property="og:url" content={homepage}/>
			// Twitter card
			<meta name="twitter:card" content="summary"/>
			<meta name="twitter:title" content={title.clone()}/>
			<meta name="twitter:description" content={description}/>
			// Apple PWA
			<meta name="apple-mobile-web-app-capable" content="yes"/>
			<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
			<meta name="apple-mobile-web-app-title" content={title.clone()}/>
			// Android PWA
			<meta name="mobile-web-app-capable" content="yes"/>
			// Microsoft tile
			<meta name="msapplication-TileColor" content="#000000"/>
			<slot/>
		</head>
	}
}

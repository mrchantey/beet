//! `<head>` widget — sensible default meta tags sourced from [`PackageConfig`].
//!
//! Web `<head>` only — non-web targets ignore the produced meta tags during
//! rendering. The meta values are sourced from [`PackageConfig`] at scene build
//! time via `#[scene(system)]`, so the same widget composition fills correctly
//! in any binary that initializes the resource (via `pkg_config!()`).
//!
//! `<head>` is split from `<header>`/`<footer>` because it targets the document
//! root, not the page body. Additional `<head>` meta tags (PWA, Apple, Twitter
//! cards…) flow in through the default slot — Bevy's `SceneList` tuple impls
//! top out at 12 children per entity, so a full social/PWA block must be passed
//! by the caller rather than baked in here.
//!
//! Each meta value is converted to a [`SmolStr`] up front so reusing it across
//! tags is a cheap Arc clone; the rsx! macro consumes each interpolation, but
//! the underlying string buffer is shared.
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
	let title = SmolStr::new(&pkg_config.title);
	let description = SmolStr::new(&pkg_config.description);
	let homepage = SmolStr::new(&pkg_config.homepage);

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
			<meta property="og:title" content={title}/>
			<meta property="og:type" content="website"/>
			<meta property="og:description" content={description}/>
			<meta property="og:url" content={homepage}/>
			<slot/>
		</head>
	}
}

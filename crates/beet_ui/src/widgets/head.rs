//! `<head>` widget — sensible default meta tags sourced from [`PackageConfig`].
//!
//! Web `<head>` only — non-web targets ignore the produced meta tags during
//! rendering. The meta values are sourced from [`PackageConfig`] at scene build
//! time via `#[template(system)]`, so the same widget composition fills correctly
//! in any binary that initializes the resource (via `pkg_config!()`).
//!
//! `<head>` is split from `<header>`/`<footer>` because it targets the document
//! root, not the page body. The full social/PWA meta block (Open Graph,
//! Twitter cards, Apple/Android PWA, Microsoft tiles) is baked in; the scene
//! `rsx!` lowering chunks >12 children into nested tuples, so the old
//! `SceneList` 12-tuple cap no longer forces them out to the caller. Extra,
//! app-specific tags still flow in through the default slot.
//!
//! Each meta value is converted to a [`SmolStr`] up front and shared across
//! tags by reference: `{&title}` lowers to `Value::new(&title)`, which clones
//! the cheap Arc-backed [`SmolStr`] at build time without moving the local, so
//! one binding feeds every tag that needs it.
//!
//! `og:site_name` is the one exception: it is bound to [`PackageConfig::title`]
//! through a [`ResourceFieldRef`] (the `@res:PackageConfig.title` form, lowered
//! here in Rust) so the site name stays live with the resource, while still
//! seeding the resolved title so SSR renders it before the first sync. The bind
//! is gated behind `json`; a no-serde build degrades to the static title.
use beet_core::prelude::*;

/// A `<head>` with sensible defaults sourced from [`PackageConfig`].
///
/// Renders charset, title, canonical, viewport (toggle `fixed_scale` for games),
/// description, version, theme-color, application-name, the core Open Graph and
/// Twitter-card tags, and the Apple/Android/Microsoft PWA meta block. Extra
/// app-specific tags can be added through the default slot.
#[template(system)]
pub fn Head(
	#[prop] fixed_scale: bool,
	/// Per-page title override; falls back to [`PackageConfig::title`].
	#[prop]
	title: Option<String>,
	/// Per-page description override; falls back to
	/// [`PackageConfig::description`].
	#[prop]
	description: Option<String>,
	pkg_config: Res<PackageConfig>,
) -> impl Bundle {
	// per-page `ArticleMeta` values override the package defaults.
	let title = SmolStr::new(title.as_deref().unwrap_or(&pkg_config.title));
	let description =
		SmolStr::new(description.as_deref().unwrap_or(&pkg_config.description));
	// homepage/version are optional: an unset field omits its tag entirely
	// rather than rendering an empty attribute.
	let homepage = pkg_config.homepage.clone();
	let version = pkg_config.version.clone();

	let scale = if fixed_scale {
		"width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
	} else {
		"width=device-width, initial-scale=1"
	};

	rsx! {
		<head>
			<meta charset="UTF-8"/>
			// child-text position needs an owned value (the block flows through
			// `into_node`, which would otherwise borrow `title`);
			// attribute positions take `{&title}` directly via `Value::new`.
			<title>{title.clone()}</title>
			{homepage.as_ref().map(|homepage| rsx!{ <link rel="canonical" href={homepage.clone()}/> })}
			<meta name="viewport" content={scale}/>
			<meta name="description" content={&description}/>
			{version.as_ref().map(|version| rsx!{ <meta name="version" content={version.clone()}/> })}
			<meta name="application-name" content={&title}/>
			<meta name="theme-color" content="#ffffff"/>
			// Open Graph
			<meta property="og:title" content={&title}/>
			<meta property="og:type" content="website"/>
			// site name stays bound to `PackageConfig.title`, not snapshotted.
			<meta property="og:site_name" {site_name_attr(&title)}/>
			<meta property="og:description" content={&description}/>
			{homepage.as_ref().map(|homepage| rsx!{ <meta property="og:url" content={homepage.clone()}/> })}
			// Twitter card
			<meta name="twitter:card" content="summary"/>
			<meta name="twitter:title" content={&title}/>
			<meta name="twitter:description" content={&description}/>
			// Apple PWA
			<meta name="apple-mobile-web-app-capable" content="yes"/>
			<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
			<meta name="apple-mobile-web-app-title" content={&title}/>
			// Android PWA
			<meta name="mobile-web-app-capable" content="yes"/>
			// Microsoft tile
			<meta name="msapplication-TileColor" content="#000000"/>
			<Slot/>
		</head>
	}
}

/// The `content` block attribute for the `og:site_name` meta: a [`Value`] seeded
/// with the resolved title (so SSR renders before any sync) plus, under `json`,
/// a [`ResourceFieldRef`] binding it to [`PackageConfig::title`] so the rendered
/// site name tracks the live resource. Without `json` it stays the static title.
///
/// This is the Rust counterpart of the `content=@res:PackageConfig.title`
/// markup form, spawning the attribute as a related entity ([`AttributeOf`] the
/// meta element) so it sits alongside the literal `property` attribute.
fn site_name_attr(title: &SmolStr) -> impl Bundle {
	let value = Value::new(title);
	OnSpawn::new(move |entity| {
		let element = entity.id();
		entity.world_scope(move |world| {
			let attr = (AttributeOf::new(element), Attribute::new("content"), value);
			// under `json` the resource bind tracks the live title; otherwise the
			// seeded value renders as a static snapshot.
			#[cfg(feature = "json")]
			world.spawn((attr, ResourceFieldRef::new("PackageConfig", "title")));
			#[cfg(not(feature = "json"))]
			world.spawn(attr);
		});
	})
}

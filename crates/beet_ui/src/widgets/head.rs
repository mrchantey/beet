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
//! Values are **site-level** by default, sourced from [`PackageConfig`], with the
//! social card — `og:title`, `og:description`, the preview image — overridden by
//! the page's own [`PageMeta`] when the caller supplies one, so a shared link
//! previews as the PAGE rather than as the site. The per-page `<title>` is owned
//! by the layout (eg [`RouteHead`](beet_router) binds it to the route's
//! `PageMeta`), so `omit_title` drops this widget's own `<title>` to keep exactly
//! one in the document.
//!
//! `og:site_name` is bound to [`PackageConfig::title`] through a
//! [`ResourceFieldRef`] (the rsx counterpart of a bsx `@res:PackageConfig.title`
//! binding) so the site name stays live with the resource. The bind is gated
//! behind `json`; a no-serde build degrades to the static title.
use crate::prelude::*;
use beet_core::prelude::*;

/// A `<head>` with sensible defaults sourced from [`PackageConfig`].
///
/// Renders charset, title, canonical, viewport (toggle `fixed_scale` for games),
/// description, version, application-name, the core Open Graph and Twitter-card
/// tags, and the Apple/Android/Microsoft PWA meta block. Extra app-specific tags
/// can be added through the default slot.
///
/// The brand-dependent tags (the social card, the theme colour) render only when
/// [`PackageConfig`] names them, so an app that has no card gets the small
/// summary preview rather than a broken image.
#[template(system)]
pub fn Head(
	#[prop] fixed_scale: bool,
	/// Omit this widget's own `<title>`, so a layout can own a single bound
	/// `<title>` (eg from the route's `PageMeta`) without a duplicate.
	#[prop]
	omit_title: bool,
	/// The page's own metadata, overriding the package defaults in the social
	/// card. Empty by default, ie a standalone `<Head/>` names the site.
	#[prop]
	meta: PageMeta,
	pkg_config: Res<PackageConfig>,
) -> impl Bundle {
	// every PWA/application value names the site, sourced from the package config.
	let title = pkg_config.title.clone();
	let description = pkg_config.description.clone();
	// ..while the social card names the PAGE where it declares itself, since that
	// is what a shared link previews.
	let card_title = meta.title.clone().unwrap_or_else(|| title.to_string());
	let card_description = meta
		.description
		.clone()
		.unwrap_or_else(|| description.to_string());
	// homepage is optional: an unset field omits its tag entirely rather than
	// rendering an empty attribute.
	let homepage = pkg_config.homepage.clone();
	let version = pkg_config.version.clone();
	// the social card and the brand tint, each omitted rather than defaulted:
	// an invented card url is a broken preview and an invented tint is another
	// brand's colour.
	let social_image = meta
		.image_url
		.as_ref()
		.map(|image| SmolStr::new(image))
		.or_else(|| pkg_config.social_image.clone());
	let theme_color = pkg_config.theme_color.clone();
	// a card only fills the large preview when there is a card to fill it with.
	let twitter_card = if social_image.is_some() {
		"summary_large_image"
	} else {
		"summary"
	};

	let scale = if fixed_scale {
		"width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
	} else {
		"width=device-width, initial-scale=1"
	};

	rsx! {
		<head>
			<meta charset="UTF-8"/>
			// the `<title>` is omittable so a layout owns the single per-route one;
			// the seeded site title is the standalone fallback.
			{(!omit_title).then(|| rsx!{ <title>{title.clone()}</title> })}
			{homepage.as_ref().map(|homepage| rsx!{ <link rel="canonical" href={homepage.clone()}/> })}
			<meta name="viewport" content={scale}/>
			<meta name="description" content={&description}/>
			<meta name="version" content={&version}/>
			<meta name="application-name" content={&title}/>
			{theme_color.as_ref().map(|color| rsx!{ <meta name="theme-color" content={color.clone()}/> })}
			// Open Graph
			<meta property="og:title" content={&card_title}/>
			<meta property="og:type" content="website"/>
			// site name stays bound to `PackageConfig.title`, not snapshotted.
			<meta property="og:site_name" {site_name_attr(&title)}/>
			<meta property="og:description" content={&card_description}/>
			{homepage.as_ref().map(|homepage| rsx!{ <meta property="og:url" content={homepage.clone()}/> })}
			{social_image.as_ref().map(|image| rsx!{ <meta property="og:image" content={image.clone()}/> })}
			// Twitter card
			<meta name="twitter:card" content={twitter_card}/>
			<meta name="twitter:title" content={&card_title}/>
			<meta name="twitter:description" content={&card_description}/>
			{social_image.as_ref().map(|image| rsx!{ <meta name="twitter:image" content={image.clone()}/> })}
			// Apple PWA
			<meta name="apple-mobile-web-app-capable" content="yes"/>
			<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
			<meta name="apple-mobile-web-app-title" content={&title}/>
			// Android PWA
			<meta name="mobile-web-app-capable" content="yes"/>
			// Microsoft tile
			{theme_color.as_ref().map(|color| rsx!{ <meta name="msapplication-TileColor" content={color.clone()}/> })}
			<Slot/>
		</head>
	}
}

/// The `content` block attribute for the `og:site_name` meta: a [`Value`] seeded
/// with [`PackageConfig::title`] plus, under `json`, a [`ResourceFieldRef`]
/// binding it to that field so the rendered site name tracks the live resource.
/// Without `json` there is no serde-backed `Value`<->reflect bridge, so it
/// degrades to the same static snapshot as every sibling meta above.
///
/// Seeding the resource's *own* value (not a per-page title) is load-bearing:
/// the bind is bidirectional, so a per-page seed would write that page's title
/// back into the shared `PackageConfig.title`, leaking it across requests.
///
/// The rsx counterpart of a bsx `content=@res:PackageConfig.title` binding; the
/// Rust macro has no `@`-binding syntax, so the bind rides the attribute entity
/// through [`Attribute::bundle_with`].
fn site_name_attr(title: &SmolStr) -> impl Bundle {
	let value = Value::new(title);
	#[cfg(feature = "json")]
	return Attribute::bundle_with(
		"content",
		value,
		ResourceFieldRef::new("PackageConfig", "title"),
	);
	#[cfg(not(feature = "json"))]
	return Attribute::bundle("content", value);
}

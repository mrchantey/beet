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
//! Values are **site-level** by default, sourced from [`PackageConfig`], with
//! everything a search result or a link preview shows — the description, the
//! social card, the canonical url, the `article:*` block — overridden by the
//! page's own [`PageMeta`] and `url` when the caller supplies them, so a shared
//! link previews as the PAGE rather than as the site. The per-page `<title>` is
//! owned by the layout (eg [`RouteHead`](beet_router) binds it to the route's
//! `PageMeta`), so `omit_title` drops this widget's own `<title>` to keep
//! exactly one in the document.
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
/// tags, the Apple/Android/Microsoft PWA meta block, and a schema.org
/// [`JsonLd`] description of the page. Extra app-specific tags can be added
/// through the default slot.
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
	/// This page's absolute url, the `<link rel="canonical">` and `og:url` a
	/// crawler resolves every relative reference against. Defaults to the
	/// [`PackageConfig`] homepage, ie the SITE, which is all a standalone
	/// `<Head/>` outside a route can honestly claim.
	#[prop(into)]
	url: Option<String>,
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
	// the page's own url, else the site's; both optional, so an unset origin
	// omits the tags entirely rather than rendering empty attributes.
	let canonical = url
		.map(SmolStr::new)
		.or_else(|| pkg_config.homepage.clone());
	let version = pkg_config.version.clone();
	// the social card and the brand tint, each omitted rather than defaulted:
	// an invented card url is a broken preview and an invented tint is another
	// brand's colour.
	let social_image = meta
		.social_image_url()
		.or_else(|| pkg_config.social_image.clone());
	let theme_color = pkg_config.theme_color.clone();
	// a dated page is an ARTICLE to a crawler and to a link preview, which is
	// what earns it a byline and a date in a search result; the `article:*`
	// block hangs off that same fact rather than off a second switch.
	let published = meta.created.map(|created| created.format_iso8601());
	let modified = meta
		.created
		.and(meta.updated)
		.map(|updated| updated.format_iso8601());
	let article_author = meta.created.and(meta.author.clone());
	let og_type = match meta.created.is_some() {
		true => "article",
		false => "website",
	};
	// an unlisted page serves to whoever holds its link and is advertised
	// nowhere, which for a crawler means `noindex` (it may still follow links).
	let noindex = meta.visibility == PageVisibility::Unlisted;
	// the same facts again as schema.org structured data, which is what a
	// search result reads for its byline and date
	let json_ld = JsonLd {
		headline: card_title.clone(),
		url: canonical.as_ref().map(|url| url.to_string()),
		published: published.clone(),
		modified: modified.clone(),
		author: article_author.clone(),
		image: social_image.clone(),
		publisher: title.clone(),
	}
	.to_json();
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
			{canonical.as_ref().map(|url| rsx!{ <link rel="canonical" href={url.clone()}/> })}
			<meta name="viewport" content={scale}/>
			// the PAGE's description where it declares one: this is the snippet a
			// search result shows, so a site-wide default under every page is a
			// site of identical results.
			<meta name="description" content={&card_description}/>
			<meta name="version" content={&version}/>
			<meta name="application-name" content={&title}/>
			{theme_color.as_ref().map(|color| rsx!{ <meta name="theme-color" content={color.clone()}/> })}
			// Open Graph
			<meta property="og:title" content={&card_title}/>
			<meta property="og:type" content={og_type}/>
			// site name stays bound to `PackageConfig.title`, not snapshotted.
			<meta property="og:site_name" {site_name_attr(&title)}/>
			<meta property="og:description" content={&card_description}/>
			{canonical.as_ref().map(|url| rsx!{ <meta property="og:url" content={url.clone()}/> })}
			{social_image.as_ref().map(|image| rsx!{ <meta property="og:image" content={image.clone()}/> })}
			// article facts, emitted only for a page that has a publication date
			{published.as_ref().map(|time| rsx!{ <meta property="article:published_time" content={time.clone()}/> })}
			{modified.as_ref().map(|time| rsx!{ <meta property="article:modified_time" content={time.clone()}/> })}
			{article_author.as_ref().map(|author| rsx!{ <meta property="article:author" content={author.clone()}/> })}
			{noindex.then(|| rsx!{ <meta name="robots" content="noindex"/> })}
			<script type="application/ld+json">{json_ld}</script>
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

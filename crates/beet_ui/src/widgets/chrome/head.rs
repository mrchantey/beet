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
//! page's own [`DocMeta`] and `url` when the caller supplies them, so a shared
//! link previews as the PAGE rather than as the site. An error page passes
//! `omit_canonical`, claiming no url at all. The per-page `<title>` is
//! owned by the layout (eg [`RouteHead`](beet_router) binds it to the route's
//! `DocMeta`), so `omit_title` drops this widget's own `<title>` to keep
//! exactly one in the document.
//!
//! The brand tint is read off the [`Theme`] resource, the palette the page's
//! own css bakes from, so the browser chrome and the page cannot disagree: a
//! `theme-color` per scheme the OS prefers, each naming its `data-scheme` so
//! [`ColorSchemeScript`] can point the browser at a scheme the page pins, and the light one
//! as the `msapplication-TileColor`, which has no media query.
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
/// The social card renders only when the caller, the page or [`PackageConfig`]
/// names one, so an app that has no card gets the small summary preview rather
/// than a broken image, and the theme colour only in a world with a [`Theme`].
#[template(system)]
pub fn Head(
	#[prop] fixed_scale: bool,
	/// Omit this widget's own `<title>`, so a layout can own a single bound
	/// `<title>` (eg from the route's `DocMeta`) without a duplicate.
	#[prop]
	omit_title: bool,
	/// The page's own metadata, overriding the package defaults in the social
	/// card. Empty by default, ie a standalone `<Head/>` names the site.
	#[prop]
	meta: DocMeta,
	/// This page's absolute url, the `<link rel="canonical">` and `og:url` a
	/// crawler resolves every relative reference against. Defaults to the
	/// [`PackageConfig`] homepage, ie the SITE, which is all a standalone
	/// `<Head/>` outside a route can honestly claim.
	#[prop]
	url: Option<Url>,
	/// The page has no address of its own, ie a `404`: omit the canonical,
	/// `og:url` and the JSON-LD `url` rather than falling back to the site.
	#[prop]
	omit_canonical: bool,
	/// Mark the page `noindex` for crawlers, ie an
	/// [`Unlisted`](RouteVisibility::Unlisted) route, which serves to whoever
	/// holds its link and is advertised nowhere (a crawler may still follow
	/// its links).
	#[prop]
	noindex: bool,
	/// The page's social card image, ahead of the page's own
	/// [`social_image_url`](DocMeta::social_image_url), ie a card generated
	/// for it.
	#[prop]
	social_image: Option<Url>,
	/// What [`social_image`](Self::social_image) shows, the alt text a link
	/// preview carries. Only ever the given image's, so it is ignored without
	/// one.
	#[prop]
	social_image_alt: Option<String>,
	pkg_config: Res<PackageConfig>,
	theme: Option<Res<Theme>>,
) -> Result<impl Bundle> {
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
	// omits the tags entirely rather than rendering empty attributes. A page
	// with no address of its own claims neither.
	let canonical = match omit_canonical {
		true => None,
		false => url.or_else(|| pkg_config.homepage.clone()),
	};
	let version = pkg_config.version.clone();
	// the social card is omitted rather than defaulted, since an invented card
	// url is a broken preview.
	let social_image_alt = social_image.as_ref().and(social_image_alt);
	let social_image = social_image
		.or_else(|| meta.social_image_url())
		.or_else(|| pkg_config.social_image.clone());
	let tints = theme.map(|theme| ThemeTints::new(&theme)).transpose()?;
	// a dated page is an ARTICLE to a crawler and to a link preview, which is
	// what earns it a byline and a date in a search result; the `article:*`
	// block hangs off that same fact rather than off a second switch.
	let published = meta
		.created
		.map(|created| created.timestamp().format_iso8601());
	let modified = meta
		.created
		.and(meta.updated)
		.map(|updated| updated.timestamp().format_iso8601());
	let article_authors = meta
		.created
		.map(|_| meta.authors.clone())
		.unwrap_or_default();
	let og_type = match meta.created.is_some() {
		true => "article",
		false => "website",
	};
	// the same facts again as schema.org structured data, which is what a
	// search result reads for its byline and date
	let json_ld = JsonLd {
		headline: card_title.clone(),
		url: canonical.clone(),
		published: published.clone(),
		modified: modified.clone(),
		authors: article_authors.clone(),
		image: social_image.clone(),
		publisher: title.clone(),
	}
	.to_json()?;
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
			{canonical.as_ref().map(|url| rsx!{ <link rel="canonical" href={url.to_string()}/> })}
			<meta name="viewport" content={scale}/>
			// the PAGE's description where it declares one: this is the snippet a
			// search result shows, so a site-wide default under every page is a
			// site of identical results.
			<meta name="description" content={&card_description}/>
			<meta name="version" content={&version}/>
			<meta name="application-name" content={&title}/>
			{tints.as_ref().map(|tints| rsx!{
				<meta name="theme-color" media="(prefers-color-scheme: light)" content={&tints.light} data-scheme="light"/>
				<meta name="theme-color" media="(prefers-color-scheme: dark)" content={&tints.dark} data-scheme="dark"/>
			})}
			// Open Graph
			<meta property="og:title" content={&card_title}/>
			<meta property="og:type" content={og_type}/>
			// site name stays bound to `PackageConfig.title`, not snapshotted.
			<meta property="og:site_name" {site_name_attr(&title)}/>
			<meta property="og:description" content={&card_description}/>
			{canonical.as_ref().map(|url| rsx!{ <meta property="og:url" content={url.to_string()}/> })}
			{social_image.as_ref().map(|image| rsx!{ <meta property="og:image" content={image.to_string()}/> })}
			{social_image_alt.as_ref().map(|alt| rsx!{ <meta property="og:image:alt" content={alt.clone()}/> })}
			// article facts, emitted only for a page that has a publication date
			{published.as_ref().map(|time| rsx!{ <meta property="article:published_time" content={time.clone()}/> })}
			{modified.as_ref().map(|time| rsx!{ <meta property="article:modified_time" content={time.clone()}/> })}
			// one tag per author, the property repeating as Open Graph's arrays do
			{article_authors.iter().map(|author| rsx!{ <meta property="article:author" content={author.clone()}/> }).collect::<Vec<_>>()}
			{noindex.then(|| rsx!{ <meta name="robots" content="noindex"/> })}
			<script type="application/ld+json">{json_ld}</script>
			// Twitter card
			<meta name="twitter:card" content={twitter_card}/>
			<meta name="twitter:title" content={&card_title}/>
			<meta name="twitter:description" content={&card_description}/>
			{social_image.as_ref().map(|image| rsx!{ <meta name="twitter:image" content={image.to_string()}/> })}
			{social_image_alt.as_ref().map(|alt| rsx!{ <meta name="twitter:image:alt" content={alt.clone()}/> })}
			// Apple PWA
			<meta name="apple-mobile-web-app-capable" content="yes"/>
			<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
			<meta name="apple-mobile-web-app-title" content={&title}/>
			// Android PWA
			<meta name="mobile-web-app-capable" content="yes"/>
			// Microsoft tile
			{tints.as_ref().map(|tints| rsx!{ <meta name="msapplication-TileColor" content={&tints.light}/> })}
			<slot/>
		</head>
	}
	.xok()
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

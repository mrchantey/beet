use crate::prelude::*;




/// A `<head>` element with sensible defaults.
#[template]
pub fn Head(
	/// Enable to force the page to be displayed at a fixed scale,
	/// disabling zooming.
	/// This is useful for games, but will reduce accessibility in documents.
	#[field(default)]
	fixed_scale: bool,
	pkg_config: Res<PackageConfig>,
) -> impl Bundle {
	let PackageConfig {
		title,
		description,
		homepage,
		version,
		..
	} = pkg_config.as_ref().clone();

	let scale = if fixed_scale {
		"width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
	} else {
		"width=device-width, initial-scale=1"
	};

	rsx! {
		<head>
		<meta charset="UTF-8">
		<title>{title.clone()}</title>
		<link rel="canonical" href={homepage.clone()}>
		<meta name="viewport" content={scale} />
		// <link rel="icon" href="https://fav.farm/🌱"/>
		<meta name="description" content={description.clone()}>
		<meta name="version" content={version.clone()}>
		// <link rel="alternate" type="application/rss+xml" title="Bevyhub Blog" href={Routes.rss} />
		// <link rel="sitemap" href="/sitemap-index.xml" />

		<meta property="og:title" content={title.clone()} />
		<meta property="og:type" content="website" />
		// <meta property="og:image" content={image} />
		<meta property="og:description" content={description.clone()} />
		<meta property="og:url" content={homepage.clone()} />

		<meta name="twitter:title" content={title.clone()}>
		<meta name="twitter:description" content={description.clone()}>
		// <meta name="twitter:image" content={image}/>
		// <meta name="twitter:card" content="summary_large_image">
		// <meta name="twitter:site" content={`@${username}`}>

		// <!-- PWA STUFF -->
		// <link rel="manifest" href="/manifest.webmanifest">
		// <!-- ios -->
		<meta name="apple-mobile-web-app-capable" content="yes">
		<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
		<meta name="apple-mobile-web-app-title" content={title.clone()}>
		// <!-- android/pwa -->
		<meta name="mobile-web-app-capable" content="yes">
		<meta name="theme-color" content="#fffff">
		<meta name="application-name" content={title.clone()}>
		// <!-- microsoft -->
		<meta name="msapplication-TileColor" content="#000000">
		<meta name="msapplication-TileImage" content="/icons/icon-144x144.png">
		<script hoist:head src="../css/initColorScheme.js" />
		<slot/>
		</head>
	}
}

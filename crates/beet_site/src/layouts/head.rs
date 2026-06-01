//! Site `<head>` and `<header>` as self-contained `#[scene(system)]` widgets.
//!
//! These emit their full structure directly rather than via slots. The
//! library's [`Head`]/[`Header`] widgets are `#[scene(system)]` and expose
//! slots for extra content, but `#[scene(system)]` + caller slot content does
//! not yet compose (the caller content clobbers the widget's structural
//! subtree), so the site assembles its own head/header instead of feeding the
//! library widgets caller content.
use crate::prelude::*;
use beet::prelude::*;

/// The site `<head>`: package-driven meta tags plus the web-only stylesheet,
/// color-scheme seed, preflight reset and favicon. The charcell renderer skips
/// `<head>`, so this is inert in the terminal.
#[scene(system)]
pub fn BeetHead(pkg: Res<PackageConfig>) -> impl Scene {
	let title = pkg.title.clone();
	let description = pkg.description.clone();
	let homepage = pkg.homepage.clone();
	rsx! {
		<head>
			<meta charset="UTF-8"/>
			<title>{title.clone()}</title>
			<link rel="canonical" href={homepage}/>
			<meta name="viewport" content="width=device-width, initial-scale=1"/>
			<meta name="description" content={description.clone()}/>
			<meta name="application-name" content={title}/>
			<meta name="theme-color" content="#ffffff"/>
			<Preflight/>
			<Stylesheet/>
			<ColorSchemeScript/>
			<link rel="icon" href="/assets/branding/favicon-32x32.png"/>
		</head>
	}
}

/// The site app bar: a title link home plus the primary nav links.
#[scene(system)]
pub fn BeetHeader(pkg: Res<PackageConfig>) -> impl Scene {
	let title = pkg.title.clone();
	rsx! {
		<header {Classes::new([classes::APP_BAR, classes::PRINT_HIDDEN])}>
			<a {Classes::new(["app-bar-title"])} href="/">{title}</a>
			<nav>
				<Link label="Docs" href={routes::docs::index()} variant=ButtonVariant::Text/>
				<Link label="Blog" href={routes::blog::index()} variant=ButtonVariant::Text/>
				<Link label="GitHub" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Text/>
			</nav>
		</header>
	}
}

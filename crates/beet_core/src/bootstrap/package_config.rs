//! The package's identity, usually declared via [`pkg_config!`].

use crate::prelude::*;

/// The identity of the package this binary was built from, usually set via
/// [`pkg_config!`].
///
/// Build-time facts only: what the package *is*, not how this process was
/// launched. Anything a launch decides (the stage, service access, ports) lives
/// on [`BootstrapConfig`], so the two cannot disagree.
///
/// This resource is required for all beet applications and should be consistent
/// across launch, server and client binaries.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct PackageConfig {
	/// The pretty name of the package, shown in titles and headers.
	pub title: SmolStr,
	/// A short description of the package, used for meta tags.
	pub description: SmolStr,
	/// The application's identity, usually set via `CARGO_PKG_NAME` in
	/// [`pkg_config!`]. The ONE location app identity lives: a deploy names its
	/// cloud resources `<app_name>--<stage>--<label>`, and the running binary
	/// resolves the same names from the same field, so the two cannot drift.
	///
	/// Always set, defaulting to [`Self::DEFAULT_APP_NAME`], so a resource name
	/// is total: a nameless app gets a generic name rather than an empty
	/// segment, and BOTH sides fall back identically so they still cannot
	/// disagree. (The live incident this guards was two INDEPENDENT
	/// derivations, not the existence of a fallback.)
	pub app_name: SmolStr,
	/// The package version, defaulting to `0.0.1` and usually overridden via
	/// `CARGO_PKG_VERSION` in [`pkg_config!`].
	pub version: SmolStr,
	/// The homepage URL, usually set via `CARGO_PKG_HOMEPAGE` in [`pkg_config!`].
	pub homepage: Option<SmolStr>,
	/// The absolute URL of the social card, the image a link preview shows.
	///
	/// Absolute because a crawler resolves it against nothing, so a relative
	/// path yields no preview at all. Unset omits the tag and the preview falls
	/// back to the small summary card.
	pub social_image: Option<SmolStr>,
	/// The brand colour a browser tints its own chrome with, ie the mobile
	/// address bar and the Microsoft tile. Unset leaves the browser default.
	pub theme_color: Option<SmolStr>,
}

/// The defaults govern unset fields for markup-only sites: a markup-declared
/// `<PackageConfig/>` is built over these when no host inserted a
/// [`pkg_config!`]. Static values only, since every launch-resolved field lives
/// on [`BootstrapConfig`].
impl Default for PackageConfig {
	fn default() -> Self {
		Self {
			title: "My Beet App".into(),
			description: "An app built with beet".into(),
			app_name: Self::DEFAULT_APP_NAME.into(),
			version: "0.0.1".into(),
			homepage: None,
			social_image: None,
			theme_color: None,
		}
	}
}

impl PackageConfig {
	/// The app identity of a package that declared none. Generic on purpose: it
	/// must not collide with a real app's name, and it must be obvious in a
	/// provisioned resource name that nobody chose it.
	pub const DEFAULT_APP_NAME: &'static str = "beet-app";

	/// The app identity, ie the `beet-site` in `beet-site--prod--analytics`.
	pub fn app_name(&self) -> &str { &self.app_name }

	/// A route path as an absolute url under [`homepage`](Self::homepage), eg
	/// `blog/ecs-router` -> `https://beet.org/blog/ecs-router`.
	///
	/// # Errors
	/// Errors when `homepage` is unset. A sitemap `<loc>` and a feed `<link>`
	/// are resolved against nothing, so a relative url there is not a degraded
	/// entry but a broken one: the loud failure names the field to set.
	pub fn absolute_url(&self, path: &str) -> Result<String> {
		let Some(homepage) = &self.homepage else {
			bevybail!(
				"cannot resolve an absolute url for '{path}': `PackageConfig.homepage` is unset, set it to this site's origin, ie `<PackageConfig homepage=\"https://example.com\"/>`"
			);
		};
		let homepage = homepage.trim_end_matches('/');
		match path.trim_matches('/') {
			"" => format!("{homepage}/"),
			path => format!("{homepage}/{path}"),
		}
		.xok()
	}
}

/// Macro to create a `PackageConfig` from compile time environment variables set by Cargo.
///
/// This saves boilerplate for various `env!` environment variables.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// let mut world = World::new();
/// world.insert_resource(PackageConfig {
/// 	title: "My Site".into(),
/// 	..pkg_config!()
/// });
/// ```
#[macro_export]
macro_rules! pkg_config {
	() => {
		$crate::prelude::PackageConfig {
			title: env!("CARGO_PKG_NAME").into(),
			description: env!("CARGO_PKG_DESCRIPTION").into(),
			app_name: env!("CARGO_PKG_NAME").into(),
			version: env!("CARGO_PKG_VERSION").into(),
			homepage: Some(env!("CARGO_PKG_HOMEPAGE").into()),
			// no cargo manifest key names a social card or a brand colour, so
			// these stay the caller's to set.
			social_image: None,
			theme_color: None,
		}
	};
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The app identity comes from the package name, the one place a deploy and
	/// the running binary both read it from.
	#[crate::test]
	fn app_name_from_the_package() {
		pkg_config!().app_name().xpect_eq("beet_core");
	}

	/// A route path joins onto the homepage with exactly one slash between them,
	/// however either side is spelled, and an unset homepage is a loud error
	/// rather than a relative url.
	#[crate::test]
	fn joins_absolute_urls() {
		let config = PackageConfig {
			homepage: Some("https://beet.org/".into()),
			..default()
		};
		config
			.absolute_url("blog/ecs-router")
			.unwrap()
			.xpect_eq("https://beet.org/blog/ecs-router".to_string());
		config
			.absolute_url("/blog/ecs-router/")
			.unwrap()
			.xpect_eq("https://beet.org/blog/ecs-router".to_string());
		// the root path is the origin itself, with its trailing slash
		config
			.absolute_url("")
			.unwrap()
			.xpect_eq("https://beet.org/".to_string());
		config
			.absolute_url("/")
			.unwrap()
			.xpect_eq("https://beet.org/".to_string());
		PackageConfig::default()
			.absolute_url("blog")
			.unwrap_err()
			.to_string()
			.xpect_contains("PackageConfig.homepage");
	}

	#[crate::test]
	fn default_shape() {
		let config = PackageConfig::default();
		config.title.as_str().xpect_eq("My Beet App");
		config
			.description
			.as_str()
			.xpect_eq("An app built with beet");
		config.app_name.as_str().xpect_eq("beet-app");
		config.version.as_str().xpect_eq("0.0.1");
		config.homepage.xpect_none();
	}

	/// A markup-declared `<PackageConfig/>` patches only its named fields over
	/// [`PackageConfig::default`]: set fields override, unset fields keep the
	/// defaults (and the optionals stay `None`).
	#[crate::test]
	fn markup_patches_over_defaults() {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world
			.resource_mut::<AppTypeRegistry>()
			.write()
			.register::<PackageConfig>();
		let nodes = BsxNode::parse_document(
			r#"<PackageConfig title="Patched"/>"#,
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		world
			.spawn_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap();

		let config = world.resource::<PackageConfig>();
		// the set field overrides the default
		config.title.as_str().xpect_eq("Patched");
		// unset fields keep their defaults
		config
			.description
			.as_str()
			.xpect_eq("An app built with beet");
		config.app_name.as_str().xpect_eq("beet-app");
		// version keeps the default since the markup did not set it
		config.version.as_str().xpect_eq("0.0.1");
	}
}

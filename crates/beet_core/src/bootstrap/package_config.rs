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
	pub app_name: Option<SmolStr>,
	/// The package version, defaulting to `0.0.1` and usually overridden via
	/// `CARGO_PKG_VERSION` in [`pkg_config!`].
	pub version: SmolStr,
	/// The homepage URL, usually set via `CARGO_PKG_HOMEPAGE` in [`pkg_config!`].
	pub homepage: Option<SmolStr>,
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
			app_name: None,
			version: "0.0.1".into(),
			homepage: None,
		}
	}
}

impl PackageConfig {
	/// The app identity, if this package declared one.
	pub fn app_name(&self) -> Option<&str> { self.app_name.as_deref() }
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
			app_name: Some(env!("CARGO_PKG_NAME").into()),
			version: env!("CARGO_PKG_VERSION").into(),
			homepage: Some(env!("CARGO_PKG_HOMEPAGE").into()),
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
		pkg_config!().app_name().unwrap().xpect_eq("beet_core");
	}

	#[crate::test]
	fn default_shape() {
		let config = PackageConfig::default();
		config.title.as_str().xpect_eq("My Beet App");
		config
			.description
			.as_str()
			.xpect_eq("An app built with beet");
		config.app_name.xpect_none();
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
		config.app_name.xpect_none();
		// version keeps the default since the markup did not set it
		config.version.as_str().xpect_eq("0.0.1");
	}
}

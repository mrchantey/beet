//! The package's identity, usually declared via [`pkg_config!`].

use crate::prelude::*;
use heck::ToKebabCase;

/// The identity of the package this binary was built from, usually set via
/// [`pkg_config!`].
///
/// Build-time facts only: what the package *is*, not how this process was
/// launched. Anything a launch decides (the stage, service access, ports) lives
/// on [`BootstrapConfig`], so the two cannot disagree; the cloud resource names
/// below combine the two, reading the stage from the process config.
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
	/// The binary name, usually set via `CARGO_PKG_NAME` in [`pkg_config!`].
	pub binary_name: Option<SmolStr>,
	/// The package version, defaulting to `0.0.1` and usually overridden via
	/// `CARGO_PKG_VERSION` in [`pkg_config!`].
	pub version: SmolStr,
	/// The homepage URL, usually set via `CARGO_PKG_HOMEPAGE` in [`pkg_config!`].
	pub homepage: Option<SmolStr>,
	/// The repository URL, usually set via `CARGO_PKG_REPOSITORY` in [`pkg_config!`].
	pub repository: Option<SmolStr>,
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
			binary_name: None,
			version: "0.0.1".into(),
			homepage: None,
			repository: None,
		}
	}
}

impl PackageConfig {
	/// Returns the binary name if set.
	pub fn binary_name(&self) -> Option<&str> { self.binary_name.as_deref() }

	/// Returns the version string.
	pub fn version(&self) -> &str { &self.version }

	/// Returns the description.
	pub fn description(&self) -> &str { &self.description }

	/// Returns the homepage URL if set.
	pub fn homepage(&self) -> Option<&str> { self.homepage.as_deref() }

	/// Returns the repository URL if set.
	pub fn repository(&self) -> Option<&str> { self.repository.as_deref() }

	/// The cloud resource name for the analytics store.
	pub fn analytics_bucket_name(&self) -> String {
		self.resource_name("analytics")
	}

	/// Prefixes the binary name and suffixes the process
	/// [`stage`](BootstrapConfig::stage) to the provided name.
	///
	/// For example `lambda` becomes `my-site-lambda-dev`.
	/// This binary-resource-stage convention must match sst config:
	/// `sst.config.ts -> new sst.aws.Function(.., {name: THIS_FIELD })`.
	pub fn resource_name(&self, descriptor: &str) -> String {
		// the cloud naming convention needs a stable prefix; fall back to the
		// title when no binary name was set.
		let binary_name = self
			.binary_name
			.as_deref()
			.unwrap_or(&self.title)
			.to_kebab_case();
		let stage = &BootstrapConfig::get().stage;
		format! {"{binary_name}--{stage}--{descriptor}"}
	}
}

impl core::fmt::Display for PackageConfig {
	/// Writes each set field as a `key: value` line, omitting unset optionals.
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		writeln!(f, "title: {}", self.title)?;
		writeln!(f, "description: {}", self.description)?;
		writeln!(f, "version: {}", self.version)?;
		for (key, value) in [
			("binary_name", &self.binary_name),
			("homepage", &self.homepage),
			("repository", &self.repository),
		] {
			if let Some(value) = value {
				writeln!(f, "{key}: {value}")?;
			}
		}
		Ok(())
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
			binary_name: Some(env!("CARGO_PKG_NAME").into()),
			version: env!("CARGO_PKG_VERSION").into(),
			homepage: Some(env!("CARGO_PKG_HOMEPAGE").into()),
			repository: option_env!("CARGO_PKG_REPOSITORY").map(|s| s.into()),
		}
	};
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The cloud name combines the package's binary name with the process stage,
	/// which defaults to `dev`.
	#[crate::test]
	fn resource_name_carries_the_stage() {
		pkg_config!()
			.resource_name("lambda")
			.xpect_eq("beet-core--dev--lambda");
	}

	#[crate::test]
	fn default_shape() {
		let config = PackageConfig::default();
		config.title.as_str().xpect_eq("My Beet App");
		config
			.description
			.as_str()
			.xpect_eq("An app built with beet");
		config.binary_name.xpect_none();
		config.version.as_str().xpect_eq("0.0.1");
		config.homepage.xpect_none();
		config.repository.xpect_none();
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
		config.binary_name.xpect_none();
		// version keeps the default since the markup did not set it
		config.version.as_str().xpect_eq("0.0.1");
	}
}

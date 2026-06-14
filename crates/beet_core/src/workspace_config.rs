//! Workspace and package configuration types.
//!
//! This module provides configuration types for beet packages and workspaces,
//! including compile-time package metadata via [`pkg_config!`] and runtime
//! workspace settings via [`WorkspaceConfig`].

use crate::prelude::*;
use heck::ToKebabCase;
use std::path::Path;
use std::str::FromStr;

/// Settings for the package, usually set via `pkg_config!()`.
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
	/// The infrastructure stage for this build.
	///
	/// Defaults to `dev` in debug builds and `prod` in release builds.
	pub stage: SmolStr,
	/// How services should be accessed.
	pub service_access: ServiceAccess,
}

/// The defaults govern unset fields for markup-only sites: a markup-declared
/// `<PackageConfig/>` is built over these when no host inserted a [`pkg_config!`].
impl Default for PackageConfig {
	fn default() -> Self {
		Self {
			title: "My Beet App".into(),
			description: "An app built with beet".into(),
			binary_name: None,
			version: "0.0.1".into(),
			homepage: None,
			repository: None,
			stage: "dev".into(),
			service_access: ServiceAccess::Local,
		}
	}
}

/// Options for how services should be accessed.
///
/// For instance, a bucket that should use the local file system during
/// development but an s3 bucket when deployed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Reflect)]
pub enum ServiceAccess {
	/// Services should be accessed via filesystem and local servers.
	Local,
	/// Services should be accessed via remote cloud services.
	Remote,
}
impl FromStr for ServiceAccess {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"local" => Ok(ServiceAccess::Local),
			"remote" => Ok(ServiceAccess::Remote),
			other => Err(format!(
				"Invalid service access: {other}, expected 'local' or 'remote'"
			)),
		}
	}
}
impl std::fmt::Display for ServiceAccess {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let s = match self {
			ServiceAccess::Local => "local",
			ServiceAccess::Remote => "remote",
		};
		write!(f, "{s}")
	}
}

impl PackageConfig {
	/// Returns the binary name if set.
	pub fn binary_name(&self) -> Option<&str> {
		self.binary_name.as_deref()
	}

	/// Returns the version string.
	pub fn version(&self) -> &str { &self.version }

	/// Returns the description.
	pub fn description(&self) -> &str { &self.description }

	/// Returns the homepage URL if set.
	pub fn homepage(&self) -> Option<&str> { self.homepage.as_deref() }

	/// Returns the repository URL if set.
	pub fn repository(&self) -> Option<&str> { self.repository.as_deref() }

	/// Returns the infrastructure stage.
	pub fn stage(&self) -> &str { &self.stage }

	/// Whether this is a production build, ie [`Self::stage`] is `prod`.
	pub fn is_prod(&self) -> bool { self.stage == "prod" }

	/// The cloud resource name for the server lambda function.
	pub fn router_lambda_name(&self) -> String { self.resource_name("router") }

	/// The cloud resource name for the static html bucket.
	pub fn html_bucket_name(&self) -> String { self.resource_name("html") }

	/// The cloud resource name for the assets bucket.
	pub fn assets_bucket_name(&self) -> String { self.resource_name("assets") }

	/// The cloud resource name for the analytics store.
	pub fn analytics_bucket_name(&self) -> String {
		self.resource_name("analytics")
	}


	/// Returns a vec of environment variables to be propagated
	/// from the parent process in compilation commands.
	#[rustfmt::skip]
	pub fn envs(&self)->Vec<(String,String)>{
		vec![
			// ("BEET_BINARY_NAME".to_string(),self.binary_name().to_string()),
			// ("BEET_VERSION".to_string(),self.version().to_string()),
			// ("BEET_DESCRIPTION".to_string(),self.description().to_string()),
			("BEET_STAGE".to_string(),self.stage().to_string()),
			("BEET_SERVICE_ACCESS".to_string(),self.service_access.to_string()),
		]
	}


	/// Prefixes the binary name and suffixes the stage to the provided name.
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
		let stage = self.stage.as_str();
		format! {"{binary_name}--{stage}--{descriptor}"}
	}
}

impl std::fmt::Display for PackageConfig {
	/// Writes each set field as a `key: value` line, omitting unset optionals.
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
		writeln!(f, "stage: {}", self.stage)
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
			stage: option_env!("BEET_STAGE").unwrap_or("dev").into(),
			service_access: option_env!("BEET_SERVICE_ACCESS")
				.map(|s| s.parse().unwrap_or(ServiceAccess::Local))
				.unwrap_or(ServiceAccess::Local),
		}
	};
}



/// Config for the scene containing all information that can be statically extracted
/// from files, including html, parsed styles etc.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct WorkspaceConfig {
	/// Filter for extracting snippets.
	///
	/// Excludes 'target' and 'node_modules' directories by default.
	pub snippet_filter: GlobFilter,
	/// Files to watch, triggering a compile-and-run of the binary with
	/// the `launch` feature on change.
	pub launch_filter: GlobFilter,
	/// Location of the `launch.ron` file to save the launch scene to.
	///
	/// See `LaunchConfig::launch_file` to direct the cli to this location.
	pub launch_file: WsPathBuf,
	/// The root directory for matching [`Self::snippet_filter`] and [`Self::launch_filter`].
	pub root_dir: WsPathBuf,
	/// The output location for the generated template scene file.
	pub snippets_dir: WsPathBuf,
	/// Location of the html directory, defaults to 'target/client'.
	pub html_dir: WsPathBuf,
	/// Location of the assets directory, defaults to 'assets'.
	pub assets_dir: WsPathBuf,
	/// Location of the analytics test directory, defaults to 'target/analytics'.
	pub analytics_dir: WsPathBuf,
	/// Directory for temp static files like client islands.
	pub client_islands_path: WsPathBuf,
}
impl Default for WorkspaceConfig {
	fn default() -> Self {
		Self {
			snippet_filter: GlobFilter::default()
				.with_exclude("*/target/*")
				.with_exclude("*/codegen/*")
				.with_exclude("*/.cache/*")
				.with_exclude("*/node_modules/*"),
			launch_filter: GlobFilter::default()
				.with_include("*/launch/*")
				.with_include("*/launch.rs"),
			launch_file: WsPathBuf::new("launch.ron"),
			root_dir: {
				#[cfg(test)]
				{
					WsPathBuf::new("tests/test_site")
				}
				#[cfg(not(test))]
				{
					WsPathBuf::default()
				}
			},
			snippets_dir: WsPathBuf::new("target/snippets"),
			html_dir: WsPathBuf::new("target/client"),
			assets_dir: WsPathBuf::new("assets"),
			analytics_dir: WsPathBuf::new("target/analytics"),
			client_islands_path: WsPathBuf::new("target/client_islands.ron"),
		}
	}
}

impl WorkspaceConfig {
	/// Creates a [`WorkspaceConfig`] for the test site directory.
	pub fn test_site() -> Self {
		let mut this = Self::default();
		this.root_dir = WsPathBuf::new("tests/test_site");
		this
	}

	/// Returns the snippets directory.
	pub fn snippets_dir(&self) -> &WsPathBuf { &self.snippets_dir }

	/// Creates a file path in the format of `path/to/file:line:col.rs`.
	///
	/// Uses [`Self::snippets_dir`] as the base.
	pub fn rsx_snippet_path(
		&self,
		path: impl AsRef<Path>,
		start_line: u32,
	) -> WsPathBuf {
		let mut path = path.as_ref().to_path_buf();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name = format!("{}:{}.rsx.ron", file_stem, start_line);
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Creates a file path in the format of `path/to/file.ron`.
	///
	/// We need the index because some files may have multiple LangSnippets
	/// and we don't always have the span.
	/// Uses [`Self::snippets_dir`] as the base.
	pub fn lang_snippet_path(&self, path: &WsPathBuf, index: u64) -> WsPathBuf {
		let mut path = path.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name = format!("{}-{}.lang.ron", file_stem, index);
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Returns `true` if the path passes the snippet filter.
	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.snippet_filter.passes(path)
	}

	/// Returns all files in the root directory that pass the snippet filter.
	pub fn get_files(&self) -> Result<Vec<AbsPathBuf>, FsError> {
		ReadDir::files_recursive(&self.root_dir.into_abs())?
			.into_iter()
			.filter(|path| self.snippet_filter.passes(path))
			.map(|path| AbsPathBuf::new(path))
			.collect()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn works() {
		pkg_config!()
			.resource_name("lambda")
			.xpect_eq("beet-core--dev--lambda");
	}

	#[crate::test]
	fn default_shape() {
		let config = PackageConfig::default();
		config.title.as_str().xpect_eq("My Beet App");
		config.description.as_str().xpect_eq("An app built with beet");
		config.binary_name.xpect_none();
		config.version.as_str().xpect_eq("0.0.1");
		config.homepage.xpect_none();
		config.repository.xpect_none();
		config.stage.as_str().xpect_eq("dev");
		config.service_access.xpect_eq(ServiceAccess::Local);
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
		let nodes = parse_document(
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
		config.description.as_str().xpect_eq("An app built with beet");
		config.stage.as_str().xpect_eq("dev");
		config.binary_name.xpect_none();
		// version keeps the default since the markup did not set it
		config.version.as_str().xpect_eq("0.0.1");
	}
}

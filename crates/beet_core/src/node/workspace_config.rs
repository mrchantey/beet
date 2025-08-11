use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use heck::ToKebabCase;
use std::path::Path;


/// A struct for propagating settings from the `launch` binary to the
/// `server` and `client` binaries via command line arguments.
/// Arguments are then applied to the [`PackageConfig`] resource.
#[derive(Debug, Default, Clone, Resource)]
#[cfg_attr(feature = "serde", derive(clap::Parser))]
pub struct ConfigArgs {
	/// The pulumi stage to use for deployments and infra resource names.
	/// By default this is set to `dev` in debug builds and `prod` in release builds.
	#[cfg_attr(feature = "serde", arg(long,default_value_t = default_stage()))]
	pub stage: String,
}

fn default_stage() -> String {
	if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		"prod".to_string()
	}
}

impl ConfigArgs {
	/// Convert this struct into a vector of command line arguments,
	/// for passing to server and client binaries.
	pub fn into_args(&self) -> Vec<String> {
		let Self { stage } = self.clone();

		vec!["--stage".to_string(), stage]
	}
}

impl Plugin for ConfigArgs {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.clone());
		app.world_mut().resource_mut::<PackageConfig>().stage =
			self.stage.clone();
	}
}

/// Settings for the package, usually set via `pkg_config!()`.
/// This resource is required for all beet applications and should be consistent
/// across launch, server and client binaries.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct PackageConfig {
	/// The name of the package set via `CARGO_PKG_NAME`
	pub name: String,
	/// The version of the package set via `CARGO_PKG_VERSION`
	pub version: String,
	/// The description of the package set via `CARGO_PKG_DESCRIPTION`
	pub description: String,
	/// The homepage URL of the package, set via `CARGO_PKG_HOMEPAGE`
	pub homepage: String,
	/// The repository URL of the package, set via `CARGO_PKG_REPOSITORY` if available
	pub repository: Option<String>,
	/// The infrastructure stage for this build,
	/// defaults to `dev` in debug builds and `prod` in release builds
	pub stage: String,
}

impl PackageConfig {
	pub fn name(&self) -> &str { &self.name }
	pub fn version(&self) -> &str { &self.version }
	pub fn description(&self) -> &str { &self.description }
	pub fn repository(&self) -> Option<&str> { self.repository.as_deref() }
	pub fn stage(&self) -> &str { &self.stage }

	pub fn default_lambda_name(&self) -> String { self.resource_name("lambda") }
	pub fn default_bucket_name(&self) -> String { self.resource_name("bucket") }

	/// Prefixes the binary name and suffixes the stage to the provided name,
	/// for example `lambda` becomes `my-site-lambda-dev`
	/// this binary-resource-stage convention must match sst config
	/// sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
	pub fn resource_name(&self, name: &str) -> String {
		let binary_name = self.name.to_kebab_case();
		let stage = self.stage.as_str();
		format! {"{binary_name}-{name}-{stage}"}
	}
}



/// Macro to create a `PackageConfig` from environment variables set by Cargo.
/// ## Example
/// ```
/// # use bevy::prelude::*;
/// # use beet_core::prelude::*;
/// let mut world = World::new();
/// world.insert_resource(pkg_config!());
/// ```
#[macro_export]
macro_rules! pkg_config {
	() => {
		$crate::prelude::PackageConfig {
			name: env!("CARGO_PKG_NAME").to_string(),
			version: env!("CARGO_PKG_VERSION").to_string(),
			description: env!("CARGO_PKG_DESCRIPTION").to_string(),
			homepage: env!("CARGO_PKG_HOMEPAGE").to_string(),
			repository: option_env!("CARGO_PKG_REPOSITORY")
				.map(|s| s.to_string()),
			stage: {
				if cfg!(debug_assertions) {
					"dev"
				} else {
					"prod"
				}
				.to_string()
			},
		}
	};
}



/// Config for the scene containing all information that can be statically extracted
/// from files, including html, parsed styles etc.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct WorkspaceConfig {
	/// Filter for extracting snippets,
	/// excludes 'target' and 'node_modules' directories by default
	#[reflect(ignore)] // TODO reflect GlobFilter
	pub filter: GlobFilter,
	/// The root directory for extracting snippets
	pub root_dir: WsPathBuf,
	/// The output location for the generated template scene file
	pub snippets_dir: WsPathBuf,
	/// Location of the html directory, defaults to 'target/client'
	pub html_dir: WsPathBuf,
	/// Directory for temp static files like client islands.
	pub client_islands_path: WsPathBuf,
}
impl Default for WorkspaceConfig {
	fn default() -> Self {
		Self {
			filter: GlobFilter::default()
				.with_exclude("*/target/*")
				.with_exclude("*/codegen/*")
				.with_exclude("*/.cache/*")
				.with_exclude("*/node_modules/*"),
			root_dir: {
				#[cfg(test)]
				{
					WsPathBuf::new("crates/beet_router/src/test_site")
				}
				#[cfg(not(test))]
				{
					WsPathBuf::default()
				}
			},
			snippets_dir: WsPathBuf::new("target/snippets"),
			html_dir: WsPathBuf::new("target/client"),
			client_islands_path: WsPathBuf::new("target/client_islands.ron"),
		}
	}
}

impl WorkspaceConfig {
	pub fn test_site() -> Self {
		let mut this = Self::default();
		this.root_dir = WsPathBuf::new("crates/beet_router/src/test_site");
		this
	}

	pub fn snippets_dir(&self) -> &WsPathBuf { &self.snippets_dir }

	/// Create a file path in the format of `path/to/file:line:col.rs`,
	/// using [`Self::snippets_dir`] as the base.
	pub fn rsx_snippet_path(&self, idx: &SnippetRoot) -> WsPathBuf {
		let mut path = idx.file.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name =
			format!("{}:{}.rsx.ron", file_stem, idx.start.to_string());
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Create a file path in the format of `path/to/file.ron`,
	/// we need the index because some files may have multiple LangSnippets
	/// and we dont always have the span.
	/// using [`Self::snippets_dir`] as the base.
	pub fn lang_snippet_path(&self, path: &WsPathBuf, index: u64) -> WsPathBuf {
		let mut path = path.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name = format!("{}-{}.lang.ron", file_stem, index);
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.filter.passes(path)
	}
	pub fn get_files(&self) -> Result<Vec<AbsPathBuf>, FsError> {
		ReadDir::files_recursive(&self.root_dir.into_abs())?
			.into_iter()
			.filter(|path| self.filter.passes(path))
			.map(|path| AbsPathBuf::new(path))
			.collect()
	}
}



#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		pkg_config!()
			.resource_name("lambda")
			.xpect()
			.to_be("beet-core-lambda-dev");
	}
}

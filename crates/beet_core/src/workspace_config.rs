// use crate::prelude::*;
use crate::prelude::*;
use heck::ToKebabCase;
use std::path::Path;
use std::str::FromStr;


/// Override settings typically set via environment variables like `BEET_STAGE`.
#[derive(Debug, Default, Clone, Resource)]
#[cfg_attr(feature = "serde", derive(clap::Parser))]
pub struct ConfigOverrides {
	/// Override the `BEET_STAGE` environment variable.
	/// The pulumi stage to use for deployments and infra resource names.
	/// By default this is set to `dev` in debug builds and `prod` in release builds.
	#[cfg_attr(feature = "serde", arg(long))]
	pub stage: Option<String>,
}


impl Plugin for ConfigOverrides {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.clone());
		if let Some(stage) = &self.stage {
			app.world_mut().resource_mut::<PackageConfig>().stage =
				stage.clone();
		}
	}
}

/// Settings for the package, usually set via `pkg_config!()`.
/// This resource is required for all beet applications and should be consistent
/// across launch, server and client binaries.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct PackageConfig {
	/// The pretty name of the package
	pub title: String,
	/// The name of the package set via `CARGO_PKG_NAME`
	pub binary_name: String,
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
	/// How services should be accessed
	pub service_access: ServiceAccess,
}

/// Options for how services should be accessed, for instance
/// a bucket that should use the local file system during development
/// but an s3 bucket when deployed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Reflect)]
pub enum ServiceAccess {
	/// Services should be accessed via filesystem and local servers
	Local,
	/// Services should be accessed via remote cloud services
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
	pub fn binary_name(&self) -> &str { &self.binary_name }
	pub fn version(&self) -> &str { &self.version }
	pub fn description(&self) -> &str { &self.description }
	pub fn repository(&self) -> Option<&str> { self.repository.as_deref() }
	pub fn stage(&self) -> &str { &self.stage }

	/// The cloud resource name for the server lambda function
	pub fn router_lambda_name(&self) -> String { self.resource_name("router") }
	/// The cloud resource name for the static html bucket
	pub fn html_bucket_name(&self) -> String { self.resource_name("html") }
	/// The cloud resource name for the assets bucket
	pub fn assets_bucket_name(&self) -> String { self.resource_name("assets") }
	/// The cloud resource name for the analytics store
	pub fn analytics_bucket_name(&self) -> String {
		self.resource_name("analytics")
	}


	/// Returns a vec of environment variables to be propagated
	/// from the parent process in compilation commands
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


	/// Prefixes the binary name and suffixes the stage to the provided name,
	/// for example `lambda` becomes `my-site-lambda-dev`
	/// this binary-resource-stage convention must match sst config
	/// sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
	pub fn resource_name(&self, descriptor: &str) -> String {
		let binary_name = self.binary_name.to_kebab_case();
		let stage = self.stage.as_str();
		format! {"{binary_name}--{stage}--{descriptor}"}
	}
}

impl std::fmt::Display for PackageConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "title: {}", self.title)?;
		writeln!(f, "binary_name: {}", self.binary_name)?;
		writeln!(f, "version: {}", self.version)?;
		writeln!(f, "description: {}", self.description)?;
		writeln!(f, "homepage: {}", self.homepage)?;
		if let Some(repo) = &self.repository {
			writeln!(f, "repository: {}", repo)?;
		} else {
			writeln!(f, "repository: None")?;
		}
		writeln!(f, "stage: {}", self.stage)?;
		Ok(())
	}
}

/// Macro to create a `PackageConfig` from compile time environment variables set by Cargo.
/// This saves boilerplate for various `env!` environment variables.
/// ## Example
/// ```
/// # use beet_core::prelude::*;
/// let mut world = World::new();
/// world.insert_resource(PackageConfig {
/// 	title: "My Site".to_string(),
/// 	..pkg_config!()
/// });
/// ```
#[macro_export]
macro_rules! pkg_config {
	() => {
		$crate::prelude::PackageConfig {
			title: env!("CARGO_PKG_NAME").to_string(),
			binary_name: env!("CARGO_PKG_NAME").to_string(),
			version: env!("CARGO_PKG_VERSION").to_string(),
			description: env!("CARGO_PKG_DESCRIPTION").to_string(),
			homepage: env!("CARGO_PKG_HOMEPAGE").to_string(),
			repository: option_env!("CARGO_PKG_REPOSITORY")
				.map(|s| s.to_string()),
			stage: option_env!("BEET_STAGE").unwrap_or("dev").to_string(),
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
	/// Location of the assets directory, defaults to 'assets'
	pub assets_dir: WsPathBuf,
	/// Location of the analytics test directory, defaults to 'target/analytics'
	pub analytics_dir: WsPathBuf,
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
	pub fn test_site() -> Self {
		let mut this = Self::default();
		this.root_dir = WsPathBuf::new("tests/test_site");
		this
	}

	pub fn snippets_dir(&self) -> &WsPathBuf { &self.snippets_dir }

	/// Create a file path in the format of `path/to/file:line:col.rs`,
	/// using [`Self::snippets_dir`] as the base.
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
			.xpect_eq("beet-core--dev--lambda");
	}
}

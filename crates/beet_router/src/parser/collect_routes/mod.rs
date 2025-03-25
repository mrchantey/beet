use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use clap::Parser;
pub use file_route::*;
pub use parse_dir_routes::*;
use std::path::PathBuf;
use sweet::prelude::*;
mod file_route;
mod parse_dir_routes;
mod wasm_routes;
pub use wasm_routes::*;


/// Will scan a directory for all public http methods in files.
/// Similar to a next-js or astro `pages` directory.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TreeFileGroup {
	/// The directory relative to the [`FileGroupConfig::root_dir`] where the files are located.
	pub src_dir: WorkspacePathBuf,
}

impl Default for TreeFileGroup {
	fn default() -> Self {
		Self {
			src_dir: WorkspacePathBuf::new("src/routes"),
		}
	}
}

impl TreeFileGroup {
	pub fn new(src_dir: WorkspacePathBuf) -> Self { Self { src_dir } }

	pub fn into_collect_routes(
		self,
		app_cx: &AppContext,
	) -> Result<CollectRoutes> {
		Ok(CollectRoutes {
			routes_dir: self.src_dir.into_canonical()?,
			pkg_name: Some(app_cx.pkg_name.clone()),
			..Default::default()
		})
	}
}



/// Parse a 'routes' dir, collecting all the routes,
/// and create a `mod.rs` which contains
/// a [ServerRoutes] struct with all the routes.
#[derive(Debug, Clone, Parser)]
pub struct CollectRoutes {
	/// Optionally specify additional tokens to be added to the top of the file.
	#[arg(long)]
	pub file_router_tokens: Option<String>,
	/// Identifier for the route type. Each route must implement
	/// [`IntoRoute<T>`] where T is this type.
	#[arg(long, default_value = "beet::prelude::StaticRoute")]
	pub route_type: String,
	/// location of the routes directory
	/// This will be used to split the path and discover the route path,
	/// the last part will be taken so it should not occur in the path twice.
	/// ✅ `src/routes/foo/bar.rs` will be `foo/bar.rs`
	/// ❌ `src/routes/foo/routes/bar.rs` will be `routes/bar.rs`
	#[arg(long, default_value = "./")]
	pub routes_dir: CanonicalPathBuf,

	/// Specify the package name so codegen can `use crate as pkg_name`
	#[arg(long)]
	pub pkg_name: Option<String>,
}

impl Default for CollectRoutes {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}



impl BuildStep for CollectRoutes {
	fn run(&self) -> Result<()> {
		self.build_and_write()?;
		Ok(())
	}
}


impl CollectRoutes {
	pub fn routes_mod_path(&self) -> PathBuf { self.routes_dir.join("mod.rs") }


	pub fn build_strings(&self) -> Result<Vec<(PathBuf, String)>> {
		let dir_routes = ReadDir {
			dirs: true,
			recursive: true,
			root: true,
			..Default::default()
		}
		.read(&self.routes_dir)?
		.into_iter()
		.map(|path| {
			let str = ParseDirRoutes::build_string(self, &path)?;
			Ok((path, str))
		})
		.collect::<Result<Vec<_>>>()?;
		Ok(dir_routes)
	}

	/// Call [Self::build_strings] then write each to disk
	pub fn build_and_write(&self) -> Result<()> {
		for (path, data) in self.build_strings()? {
			let mod_path = path.join("mod.rs");
			FsExt::write(mod_path, &data)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let config = CollectRoutes {
			routes_dir: WorkspacePathBuf::new(
				"crates/beet_router/src/test_site/routes",
			)
			.into_canonical()
			.unwrap(),
			..Default::default()
		};

		let paths = config.build_strings().unwrap();
		expect(paths.len()).to_be(2);
		// println!("{:#?}", paths);
	}
}

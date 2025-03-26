use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use clap::Parser;
pub use file_route::*;
pub use parse_dir_routes::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use sweet::prelude::*;
mod file_route;
mod parse_dir_routes;
mod wasm_routes;
pub use wasm_routes::*;


/// Will scan a directory for all public http methods in files.
/// Similar to a next-js or astro `pages` directory.
/// Parse a 'routes' dir, collecting all the routes,
/// and create a `mod.rs` which contains
/// a [ServerRoutes] struct with all the routes.
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct BuildFileRoutes {
	/// Optionally specify additional tokens to be added to the top of the file.
	#[arg(long)]
	pub file_router_tokens: Option<String>,
	/// Identifier for the route type. Each route must implement
	/// [`IntoRoute<T>`] where T is this type.
	#[arg(long, default_value = "beet::prelude::StaticRoute")]
	pub route_type: String,
	/// Specify a package name to support importing of local components.
	/// This will be assigned automatically by the [`AppConfig`] if not provided.
	#[arg(long)]
	pub pkg_name: Option<String>,
	/// location of the routes directory
	/// This will be used to split the path and discover the route path,
	/// the last part will be taken so it should not occur in the path twice.
	/// ✅ `src/routes/foo/bar.rs` will be `foo/bar.rs`
	/// ❌ `src/routes/foo/routes/bar.rs` will be `routes/bar.rs`
	#[command(flatten)]
	pub files: FileGroup,
	/// Specify the package name so codegen can `use crate as pkg_name`
	#[arg(long, default_value = "src/routes/mod.rs")]
	pub codegen_file: WorkspacePathBuf,
}

impl Default for BuildFileRoutes {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}



impl BuildStep for BuildFileRoutes {
	fn run(&self) -> Result<()> {
		self.build_and_write()?;
		Ok(())
	}
}


impl BuildFileRoutes {
	pub fn build_strings(&self) -> Result<Vec<(PathBuf, String)>> {
		let canonical_src = self.files.src.into_canonical()?;
		let canonical_src_str = canonical_src.to_string_lossy();

		let dir_routes = ReadDir {
			dirs: true,
			recursive: true,
			root: true,
			..Default::default()
		}
		.read(&canonical_src)?
		.into_iter()
		.map(|path| {
			let str =
				ParseDirRoutes::build_string(self, &path, &canonical_src_str)?;
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
		let config = BuildFileRoutes {
			files: "crates/beet_router/src/test_site/routes".into(),
			..Default::default()
		};

		let paths = config.build_strings().unwrap();
		expect(paths.len()).to_be(2);
		// println!("{:#?}", paths);
	}
}

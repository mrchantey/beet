pub use crate::prelude::*;
use anyhow::Result;
use clap::Parser;
pub use file_route::*;
pub use parse_dir_routes::*;
use std::path::PathBuf;
use sweet::prelude::*;
mod file_route;
mod parse_dir_routes;

/// Parse a 'routes' dir, collecting all the routes,
/// and create a `mod.rs` which contains
/// a [ServerRoutes] struct with all the routes.
#[derive(Debug, Clone, Parser)]
pub struct CollectRoutes {
	/// Optionally specify additional tokens to be added to the top of the file.
	#[arg(long)]
	pub file_router_tokens: Option<String>,
	/// Identifier for the router. The router must have
	/// where T can be a type or trait for each route on the site.
	#[arg(long, default_value = "beet::prelude::DefaultFileRouter")]
	pub file_router_ident: String,
	/// location of the routes directory
	/// This will be used to split the path and discover the route path,
	/// the last part will be taken so it should not occur in the path twice.
	/// ✅ `src/routes/foo/bar.rs` will be `foo/bar.rs`
	/// ❌ `src/routes/foo/routes/bar.rs` will be `routes/bar.rs`
	#[arg(long, default_value = "src/routes")]
	pub routes_dir: PathBuf,
}

impl Default for CollectRoutes {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
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
		let routes_dir = FsExt::workspace_root()
			.join("crates/beet_router/src/test_site/routes");
		let config = CollectRoutes {
			routes_dir,
			..Default::default()
		};

		let paths = config.build_strings().unwrap();
		expect(paths.len()).to_be(2);
		// println!("{:#?}", paths);
	}
}

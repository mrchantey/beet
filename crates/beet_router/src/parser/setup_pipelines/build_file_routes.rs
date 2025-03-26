use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use beet_rsx::rsx::RsxPipelineTarget;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use sweet::prelude::*;


/// Will scan a directory for all public http methods in files.
/// Similar to a next-js or astro `pages` directory.
/// Parse a 'routes' dir, collecting all the routes,
/// and create a `mod.rs` which contains
/// a [ServerRoutes] struct with all the routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFileRoutes {
	/// location of the routes directory
	pub file_group: FileGroup,
	pub group_to_funcs: FileGroupToFuncs,
	pub funcs_to_routes: FileFuncsToRouteTypes,
	/// Optionally specify additional tokens to be added to the top of the file.
	pub file_router_tokens: Option<String>,
	/// Identifier for the route type. Each route must implement
	/// [`IntoRoute<T>`] where T is this type.
	pub route_type: String,
	/// Specify a package name to support importing of local components.
	/// This will be assigned automatically by the [`AppConfig`] if not provided.
	pub pkg_name: Option<String>,
	/// Specify the package name so codegen can `use crate as pkg_name`
	pub output: WorkspacePathBuf,
}

impl Default for BuildFileRoutes {
	fn default() -> Self {
		Self {
			file_group: FileGroup::default(),
			group_to_funcs: Default::default(),
			funcs_to_routes: Default::default(),
			file_router_tokens: None,
			route_type: "beet::prelude::StaticRoute".into(),
			pkg_name: None,
			output: "src/routes/mod.rs".into(),
		}
	}
}



impl BuildStep for BuildFileRoutes {
	fn run(&self) -> Result<()> {
		self.file_group.clone().pipe(self.group_to_funcs.clone())?;


		self.build_and_write()?;
		Ok(())
	}
}


impl BuildFileRoutes {
	pub fn build_strings(&self) -> Result<Vec<(PathBuf, String)>> {
		let canonical_src = &self.file_group.src;
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
			file_group: FileGroup::new_workspace_rel(
				"crates/beet_router/src/test_site/routes",
			)
			.unwrap(),
			..Default::default()
		};

		let paths = config.build_strings().unwrap();
		expect(paths.len()).to_be(2);
		// println!("{:#?}", paths);
	}
}

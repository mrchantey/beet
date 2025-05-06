use super::FileTemplates;
use super::FileToTemplates;
use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use clap::Parser;
use rapidhash::RapidHashMap;
use rayon::iter::*;
use std::path::PathBuf;
use sweet::prelude::*;

/// Build an [`RsxTemplateMap`] and write it to a file
#[derive(Debug, Clone, Parser)]
pub struct BuildTemplateMap {
	/// File or directory to watch and create templates for
	//  TODO this might be better as an include pattern
	#[arg(long, default_value = "./")]
	pub templates_root_dir: PathBuf,
	/// Location of the `rsx-templates.ron` file
	#[arg(long, default_value = default_paths::RSX_TEMPLATES)]
	pub templates_map_path: PathBuf,
	/// Output the contents of the `rsx-templates.ron` file to stdout
	/// on change
	#[arg(short, long)]
	pub templates_map_stdout: bool,
	/// directly serialize the ron tokens when building templates
	/// instead of parsing via [ron::ser::to_string_pretty]
	#[arg(long)]
	pub minify_templates: bool,
}

fn default_filter() -> GlobFilter {
	GlobFilter::default()
		.with_exclude("*/target/*")
		.with_exclude("*/node_modules/*")
}

impl Default for BuildTemplateMap {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}

impl BuildStep for BuildTemplateMap {
	fn run(&self) -> Result<()> { self.build_and_write() }
}

impl BuildTemplateMap {
	pub fn new(src: impl Into<PathBuf>) -> Self {
		Self::new_with_dst(src, default_paths::RSX_TEMPLATES)
	}
	pub fn new_with_dst(
		src: impl Into<PathBuf>,
		dst: impl Into<PathBuf>,
	) -> Self {
		Self {
			minify_templates: false,
			templates_root_dir: src.into(),
			templates_map_path: dst.into(),
			templates_map_stdout: false,
		}
	}

	pub fn build_and_write(&self) -> Result<()> {
		let map = self.build_template_map()?;
		// its already minified, so we prettify if false
		let map_str = if self.minify_templates {
			ron::ser::to_string(&map)
		} else {
			ron::ser::to_string_pretty(&map, Default::default())
		}?;
		if self.templates_map_stdout {
			println!("{}", map_str);
		}
		FsExt::write(&self.templates_map_path, &map_str)?;
		Ok(())
	}


	pub fn build_template_map(&self) -> Result<RsxTemplateMap> {
		let filter = default_filter();
		let items = ReadDir::files_recursive(&self.templates_root_dir)?
			.into_par_iter()
			.filter(|path| filter.passes(path))
			.map(|path| {
				WorkspacePathBuf::new_from_canonicalizable(path)?
					.xpipe(FileToTemplates)
			})
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.map(
				|FileTemplates {
				     location,
				     template_node,
				 }| { (location, template_node) },
			)
			.collect::<RapidHashMap<_, _>>();

		let root = WorkspacePathBuf::new_from_canonicalizable(
			&self.templates_root_dir,
		)?;

		RsxTemplateMap {
			root,
			templates: items,
		}
		.xok()
	}
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use std::path::PathBuf;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let src = WorkspacePathBuf::new("ws_rsx/beet_router/src/test_site")
			.into_abs()
			.unwrap();

		let map = BuildTemplateMap {
			templates_root_dir: src.to_path_buf(),
			templates_map_path: PathBuf::default(),
			..Default::default()
		}
		.build_template_map()
		.unwrap();

		expect(map.root())
			.to_be(&WorkspacePathBuf::new_from_canonicalizable(src).unwrap());
		expect(map.templates.len()).to_be_greater_or_equal_to(8);
		// println!("{:#?}", map);
	}

	/// it asserts that the process of loading tokens from macros
	/// matches the process of loading tokens from the file system.
	/// There are several ways this can go wrong:
	/// - compile time hasher entropy differs from runtime
	/// - macros discard whitespace but files do not
	#[sweet::test]
	async fn builds() {
		use beet_rsx::prelude::*;

		let src = WorkspacePathBuf::new("ws_rsx/beet_router/src/test_site")
			.into_abs()
			.unwrap();
		let builder = BuildTemplateMap::new(src.as_path());


		// 2. build, parse and compare
		let map = builder.build_template_map().unwrap();

		// println!("wrote to {}\n{:#?}", builder.dst.display(), map);
		// println!("TEMPLATE_MAP::::{:#?}", map);

		let rsx = &beet_router::test_site::pages::collect()[0];
		let node = (rsx.func)().await.unwrap();
		let node1 = map.templates.get(&node.location().unwrap()).unwrap();
		let RsxTemplateNode::Component {
			tracker: tracker1, ..
		} = &node1
		else {
			panic!();
		};
		let RsxNode::Component(RsxComponent {
			tracker: tracker2, ..
		}) = &node
		else {
			panic!();
		};
		expect(tracker1).to_be(tracker2);

		// println!("RSX:::: {:#?}", rsx);}
	}
}

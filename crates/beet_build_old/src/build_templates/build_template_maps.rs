use super::error::Error;
use super::error::Result;
use crate::prelude::*;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use clap::Parser;
use rayon::iter::*;
use std::path::Path;
use std::path::PathBuf;
use sweet::prelude::*;

/// Build template maps and write to a file, maps include:
/// - [`NodeTemplateMap`]
/// - [`LangTemplateMap`]
#[derive(Debug, Clone, Parser)]
pub struct BuildTemplateMaps {
	/// File or directory to watch and create templates for
	//  TODO this might be better as an include pattern
	#[arg(long, default_value = "./")]
	pub templates_root_dir: PathBuf,
	/// Location of the [`NodeTemplateMap`] file
	#[arg(long, default_value = default_paths::NODE_TEMPLATE_MAP)]
	pub node_templates_path: PathBuf,
	/// Location of the [`LangTemplateMap`] file
	#[arg(long, default_value = default_paths::LANG_TEMPLATE_MAP)]
	pub lang_templates_path: PathBuf,
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

impl Default for BuildTemplateMaps {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}

impl BuildStep for BuildTemplateMaps {
	fn run(&self) -> anyhow::Result<()> { self.build_and_write()?.xok() }
}

impl BuildTemplateMaps {
	pub fn new(src: impl Into<PathBuf>) -> Self {
		Self {
			templates_root_dir: src.into(),
			..Default::default()
		}
	}

	pub fn build_and_write(&self) -> Result<()> {
		let (node_map, lang_map) = self.build_template_maps()?;
		self.write(&self.node_templates_path, &node_map)?;
		self.write(&self.lang_templates_path, &lang_map)?;
		Ok(())
	}
	fn write(
		&self,
		path: &Path,
		map: impl serde::ser::Serialize,
	) -> Result<()> {
		let map_str = if self.minify_templates {
			ron::ser::to_string(&map)
		} else {
			ron::ser::to_string_pretty(&map, Default::default())
		}
		.map_err(|err| Error::serialize_ron(path, err))?;
		if self.templates_map_stdout {
			println!("{}", map_str);
		}

		FsExt::write(path, &map_str).map_err(Error::File)?;
		Ok(())
	}


	pub fn build_template_maps(
		&self,
	) -> Result<(NodeTemplateMap, LangTemplateMap)> {
		let filter = default_filter();
		let (node_templates, lang_templates) =
			ReadDir::files_recursive(&self.templates_root_dir)
				.map_err(Error::File)?
				.into_par_iter()
				.filter(|path| filter.passes(path))
				.map(|path| {
					let path = WorkspacePathBuf::new_cwd_rel(path)
						.map_err(Error::File)?;
					path.clone().xpipe(FileToTemplates).map_err(|err| {
						Error::file_to_templates(&path, err.to_string())
					})
				})
				.collect::<Result<Vec<_>>>()?
				.into_iter()
				.fold(
					(Vec::new(), Vec::new()),
					|(mut node_templates, mut lang_templates), file| {
						node_templates.extend(file.node_templates);
						lang_templates.extend(file.lang_templates);
						(node_templates, lang_templates)
					},
				);

		let root = WorkspacePathBuf::new_cwd_rel(&self.templates_root_dir)
			.map_err(Error::File)?;
		#[allow(unused_mut)]
		let mut lang_template_map = lang_templates.xpipe(CollectLangTemplates)?;
		#[cfg(feature = "style")]
		{
			lang_template_map =
				lang_template_map.xpipe(ParseComponentStyles::default())?;
		}

		(
			NodeTemplateMap::new(root, node_templates),
			lang_template_map,
		)
			.xok()
	}
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
	use std::path::PathBuf;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let src = WorkspacePathBuf::new("crates/beet_router/src/test_site")
			.into_abs()
			.unwrap();

		let (node_map, lang_map) = BuildTemplateMaps {
			templates_root_dir: src.to_path_buf(),
			node_templates_path: PathBuf::default(),
			..Default::default()
		}
		.build_template_maps()
		.unwrap();

		expect(node_map.root())
			.to_be(&WorkspacePathBuf::new_cwd_rel(src).unwrap());
		expect(node_map.len()).to_be_greater_or_equal_to(8);
		expect(lang_map.len()).to_be_greater_or_equal_to(2);
	}

	/// it asserts that the process of loading tokens from macros
	/// matches the process of loading tokens from the file system.
	/// There are several ways this can go wrong:
	/// - compile time hasher entropy differs from runtime
	/// - macros discard whitespace but files do not
	#[sweet::test]
	async fn builds() {
		use beet_rsx::prelude::*;

		let src = WorkspacePathBuf::new("crates/beet_router/src/test_site")
			.into_abs()
			.unwrap();
		let builder = BuildTemplateMaps::new(src.as_path());


		// 2. build, parse and compare
		let (node_map, _lang_map) = builder.build_template_maps().unwrap();

		// println!("wrote to {}\n{:#?}", builder.dst.display(), map);

		let rsx = &beet_router::test_site::pages::collect()[0];
		let node = (rsx.func)().await.unwrap();
		// println!("Template Map: {:#?}", map);
		// println!("location: {:#?}", node.location());
		let node1 = node_map.templates.get(&node.span()).unwrap();
		let WebNodeTemplate::Component {
			tracker: tracker1, ..
		} = &node1
		else {
			panic!();
		};
		let WebNode::Component(RsxComponent {
			tracker: tracker2, ..
		}) = &node
		else {
			panic!();
		};
		expect(tracker1).to_be(tracker2);

		// println!("RSX:::: {:#?}", rsx);}
	}
}

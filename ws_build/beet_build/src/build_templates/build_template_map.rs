use super::FileTemplates;
use super::FileToTemplates;
use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use clap::Parser;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;
use sweet::prelude::FsExt;
use sweet::prelude::ReadDir;
use sweet::prelude::WorkspacePathBuf;

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
	/// dont check if the ron file is valid
	#[arg(long)]
	pub skip_ron_check: bool,
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
			skip_ron_check: false,
		}
	}

	pub fn build_and_write(&self) -> Result<()> {
		let map_tokens = self.build_ron()?;
		let mut map_str = map_tokens.to_string();
		// its already minified, so we prettify if false
		if self.minify_templates == false {
			let map = ron::de::from_str::<RsxTemplateMap>(&map_str)
				.map_err(|e| ron_cx_err(e, &map_str))?;
			map_str = ron::ser::to_string_pretty(&map, Default::default())?;
		}
		if self.templates_map_stdout {
			println!("{}", map_str);
		}
		FsExt::write(&self.templates_map_path, &map_str)?;
		Ok(())
	}


	pub fn build_ron(&self) -> Result<TokenStream> {
		let items = ReadDir::files_recursive(&self.templates_root_dir)?
			.into_iter()
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
				     rsx_ron,
				     style_ron: _,
				 }| {
					let line = Literal::u32_unsuffixed(location.line);
					let col = Literal::u32_unsuffixed(location.col);
					let file = location.file.to_string_lossy();
					let kvp_tokens = quote! {
						RsxMacroLocation(
							file: (#file),
							line: #line,
							col: #col
						):#rsx_ron
					};
					// validate the rsx ron
					if !self.skip_ron_check {
						let str = rsx_ron.to_string();
						let str = str.trim();
						let _parsed =
							ron::de::from_str::<RsxTemplateNode>(&str)
								.map_err(|e| ron_cx_err(e, &str))?;
					}
					Ok(kvp_tokens)
				},
			)
			.collect::<Result<Vec<_>>>()?;

		let root = WorkspacePathBuf::new_from_canonicalizable(
			&self.templates_root_dir,
		)?;
		let root = root.to_string_lossy();

		let map = quote! {
			RsxTemplateMap(
				root: (#root),
				templates: {#(#items),*}
			)
		};
		Ok(map)
	}
}

/// how many leading and trailing characters to show in the context of the error
const CX_SIZE: usize = 8;
/// A ron deserialization error with the context of the file and line
fn ron_cx_err(e: ron::error::SpannedError, str: &str) -> anyhow::Error {
	let start = e.position.col.saturating_sub(CX_SIZE);
	let end = e.position.col.saturating_add(CX_SIZE);
	let cx = if e.position.line == 1 {
		str[start..end].to_string()
	} else {
		let lines = str.lines().collect::<Vec<_>>();
		let str = lines.get(e.position.line - 1).unwrap_or(&"");
		str[start..end].to_string()
	};
	return anyhow::anyhow!(
		"Failed to parse RsxTemplate from string\nError: {}\nContext: {}\nFull: {}",
		e.code,
		cx,
		str
	);
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use std::path::PathBuf;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let src = WorkspacePathBuf::new("ws_rsx/beet_router/src/test_site")
			.into_abs()
			.unwrap();

		let file = BuildTemplateMap {
			templates_root_dir: src.to_path_buf(),
			templates_map_path: PathBuf::default(),
			..Default::default()
		}
		.build_ron()
		.unwrap()
		.to_string();

		let map: RsxTemplateMap = ron::de::from_str(&file).unwrap();
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
		let tokens = builder.build_ron().unwrap();
		let map: RsxTemplateMap =
			ron::de::from_str(&tokens.to_string()).unwrap();

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

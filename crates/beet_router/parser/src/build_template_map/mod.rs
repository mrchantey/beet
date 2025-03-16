pub use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use clap::Parser;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;
use sweet::prelude::FsExt;
use sweet::prelude::ReadDir;
use sweet::prelude::ReadFile;
use syn::spanned::Spanned;
use syn::visit::Visit;
mod hash_file;
mod template_watcher;
pub use hash_file::*;
pub use template_watcher::*;


#[derive(Debug, Parser)]
pub struct BuildTemplateMap {
	/// File or directory to watch and create templates for
	//  TODO this might be better as an include pattern
	#[arg(long, default_value = "./")]
	pub templates_root_dir: PathBuf,
	/// Location of the `rsx-templates.ron` file
	#[arg(long, default_value = Self::DEFAULT_TEMPLATES_MAP_PATH)]
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

impl Default for BuildTemplateMap {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}

impl BuildTemplateMap {
	pub const DEFAULT_TEMPLATES_MAP_PATH: &'static str =
		"target/rsx-templates.ron";

	pub fn new(src: impl Into<PathBuf>) -> Self {
		Self::new_with_dst(src, Self::DEFAULT_TEMPLATES_MAP_PATH)
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
		let map_tokens = self.build_ron()?;
		let mut map_str = map_tokens.to_string();
		// its already minified, so we prettify if false
		if self.minify_templates == false {
			let map = ron::de::from_str::<RsxTemplateMap>(&map_str)?;
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
			.map(|path| self.file_templates(path))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.map(|(RsxMacroLocation { file, line, col }, tokens)| {
				let line = Literal::usize_unsuffixed(line);
				let col = Literal::usize_unsuffixed(col);
				quote! {
					RsxMacroLocation(
						file: #file,
						line: #line,
						col: #col
					):#tokens
				}
			});

		let root =
			ron::ser::to_string(&self.templates_root_dir.canonicalize()?)?;
		let root: TokenStream = root
			.parse()
			.map_err(|_| anyhow::anyhow!("Failed to parse root path"))?;

		let map = quote! {
			RsxTemplateMap(
				root: #root,
				templates: {#(#items),*}
			)
		};
		Ok(map)
	}

	/// get all the rsx macros in this file.
	/// If it doesnt have a rust extension an empty vec is returned
	fn file_templates(
		&self,
		path: PathBuf,
	) -> Result<Vec<(RsxMacroLocation, TokenStream)>> {
		if path.extension().map_or(false, |ext| ext == "rs") {
			let file = ReadFile::to_string(&path)?;
			let file = syn::parse_file(&file)?;
			let mac = syn::parse_quote!(rsx);
			let mut visitor = RsxSynVisitor::new(path.to_string_lossy(), mac);

			visitor.visit_file(&file);
			Ok(visitor.templates)
		} else {
			Ok(Default::default())
		}
	}
}

#[derive(Debug)]
struct RsxSynVisitor {
	file: String,
	templates: Vec<(RsxMacroLocation, TokenStream)>,
	mac: syn::Ident,
}
impl RsxSynVisitor {
	pub fn new(file: impl Into<String>, mac: syn::Ident) -> Self {
		Self {
			file: file.into(),
			templates: Default::default(),
			mac,
		}
	}
}


impl<'a> Visit<'a> for RsxSynVisitor {
	fn visit_macro(&mut self, mac: &syn::Macro) {
		if mac
			.path
			.segments
			.last()
			.map_or(false, |seg| seg.ident == self.mac)
		{
			// use the span of the inner tokens to match the behavior of
			// the rsx! macro
			let span = mac.tokens.span();
			let start = span.start();
			let loc =
				RsxMacroLocation::new(&self.file, start.line, start.column);
			let tokens = RstmlToRsxTemplate::default()
				.map_tokens(mac.tokens.clone(), &self.file);
			self.templates.push((loc, tokens));
		}
	}
}


#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use crate::prelude::*;
	use beet_rsx::rsx::RsxTemplateMap;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let src =
			FsExt::workspace_root().join("crates/beet_router/src/test_site");

		let file = BuildTemplateMap {
			templates_root_dir: src.clone(),
			templates_map_path: PathBuf::default(),
			..Default::default()
		}
		.build_ron()
		.unwrap()
		.to_string();
		let map: RsxTemplateMap = ron::de::from_str(&file).unwrap();
		expect(map.root).to_be(src);
		expect(map.templates.len()).to_be(4);
		// println!("{:#?}", map);
	}
}

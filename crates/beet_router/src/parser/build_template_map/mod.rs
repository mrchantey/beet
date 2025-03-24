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
use sweet::prelude::WorkspacePathBuf;
use syn::spanned::Spanned;
use syn::visit::Visit;
mod hash_file;
mod template_watcher;
pub use hash_file::*;
pub use template_watcher::*;

/// Build an [`RsxTemplateMap`] and write it to a file
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
	/// dont check if the ron file is valid
	#[arg(long)]
	pub skip_ron_check: bool,
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
				let path = WorkspacePathBuf::new_from_current_directory(path)?;
				let templates = self.file_templates(path)?;
				Ok(templates)
			})
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.map(|(RsxMacroLocation { file, line, col }, tokens)| {
				let line = Literal::usize_unsuffixed(line);
				let col = Literal::usize_unsuffixed(col);
				let file = file.to_string_lossy();
				let kvp_tokens = quote! {
					RsxMacroLocation(
						file: (#file),
						line: #line,
						col: #col
					):#tokens
				};
				if !self.skip_ron_check {
					let str = tokens.to_string();
					let str = str.trim();
					let _parsed = ron::de::from_str::<RsxTemplateRoot>(&str)
						.map_err(|e| ron_cx_err(e, &str))?;
				}
				Ok(kvp_tokens)
			})
			.collect::<Result<Vec<_>>>()?;

		let root = WorkspacePathBuf::new_from_current_directory(
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

	/// get all the rsx macros in this file.
	/// If it doesnt have a rust extension an empty vec is returned
	fn file_templates(
		&self,
		path: WorkspacePathBuf,
	) -> Result<Vec<(RsxMacroLocation, TokenStream)>> {
		if path.extension().map_or(false, |ext| ext == "rs") {
			let canonical_path = path.into_canonical()?;
			let file = ReadFile::to_string(&canonical_path)?;
			let file = syn::parse_file(&file)?;
			let mac = syn::parse_quote!(rsx);
			let mut visitor = RsxSynVisitor::new(path, mac);

			visitor.visit_file(&file);
			Ok(visitor.templates)
		} else {
			Ok(Default::default())
		}
	}
}

/// A ron deserialization error with the context of the file and line
const CX_SIZE: usize = 8;
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

#[derive(Debug)]
struct RsxSynVisitor {
	/// Used for creating [`RsxMacroLocation`] in several places
	file: WorkspacePathBuf,
	templates: Vec<(RsxMacroLocation, TokenStream)>,
	mac: syn::Ident,
}
impl RsxSynVisitor {
	pub fn new(file: WorkspacePathBuf, mac: syn::Ident) -> Self {
		Self {
			file,
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
			let loc = RsxMacroLocation::new(
				self.file.clone(),
				start.line,
				start.column,
			);
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
	#[cfg(not(target_arch = "wasm32"))]
	fn works() {
		let src = WorkspacePathBuf::new("crates/beet_router/src/test_site")
			.into_canonical()
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
			.to_be(&WorkspacePathBuf::new_from_current_directory(src).unwrap());
		expect(map.templates.len()).to_be(4);
		// println!("{:#?}", map);
	}
}

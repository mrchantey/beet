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
pub use hash_file::*;


#[derive(Debug, Parser)]
pub struct BuildRsxTemplateMap {
	#[arg(long, default_value = "src")]
	pub src: PathBuf,
	// keep default in sync with StaticFileRouter
	#[arg(long, default_value = "target/rsx-templates.ron")]
	pub dst: PathBuf,
}



impl Default for BuildRsxTemplateMap {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}

impl BuildRsxTemplateMap {
	pub fn build_and_write(&self) -> Result<()> {
		let ron = self.build_ron()?;
		FsExt::write(&self.dst, &ron.to_string())?;
		Ok(())
	}


	pub fn build_ron(&self) -> Result<TokenStream> {
		let items = ReadDir::files_recursive(&self.src)?
			.into_iter()
			.map(|path| self.file_templates(path))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.map(|(RsxLocation { file, line, col }, tokens)| {
				let line = Literal::usize_unsuffixed(line);
				let col = Literal::usize_unsuffixed(col);
				quote! {
					RsxLocation(
						file: #file,
						line: #line,
						col: #col
					):#tokens
				}
			});

		let map = quote! {
			RsxTemplateMap({#(#items),*})
		};
		Ok(map)
	}


	fn file_templates(
		&self,
		path: PathBuf,
	) -> Result<Vec<(RsxLocation, TokenStream)>> {
		let file = ReadFile::to_string(&path)?;
		let file = syn::parse_file(&file)?;
		let mac = syn::parse_quote!(rsx);
		let mut visitor = RsxVisitor::new(path.to_string_lossy(), mac);

		visitor.visit_file(&file);
		Ok(visitor.templates)
	}
}

#[derive(Debug)]
struct RsxVisitor {
	file: String,
	templates: Vec<(RsxLocation, TokenStream)>,
	mac: syn::Ident,
}
impl RsxVisitor {
	pub fn new(file: impl Into<String>, mac: syn::Ident) -> Self {
		Self {
			file: file.into(),
			templates: Default::default(),
			mac,
		}
	}
}


impl<'a> Visit<'a> for RsxVisitor {
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
			let loc = RsxLocation::new(&self.file, start.line, start.column);
			let tokens = RstmlToRsxTemplate::default()
				.map_tokens(mac.tokens.clone(), &self.file);
			self.templates.push((loc, tokens));
		}
	}
}

pub use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::RstmlToRsxTemplate;
use clap::Parser;
use proc_macro2::TokenStream;
use std::path::PathBuf;
use sweet::prelude::ReadDir;
use sweet::prelude::ReadFile;
use syn::visit::Visit;
// use syn::File;


#[derive(Debug, Parser)]
pub struct BuildRsxTemplates {
	#[arg(long, default_value = "src")]
	pub src: PathBuf,
	#[arg(long, default_value = "target/rsx-templates.bin")]
	pub dst: PathBuf,
}



impl Default for BuildRsxTemplates {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}

impl BuildRsxTemplates {
	pub fn run(&self) -> Result<()> {
		ReadDir::files_recursive(&self.src)?
			.into_iter()
			.map(|path| self.file_templates(path))
			.collect::<Result<Vec<_>>>()?;
		Ok(())
	}


	fn file_templates(&self, path: PathBuf) -> Result<()> {
		let file = ReadFile::to_string(&path)?;
		let file = syn::parse_file(&file)?;
		let mut visitor = RsxVisitor::default();
		visitor.visit_file(&file);
		println!("{:?}", path);
		println!(
			"{}",
			visitor
				.templates
				.iter()
				.map(|t| t.to_string())
				.collect::<Vec<_>>()
				.join("\n")
		);

		// Ok((path, file))

		// let file = syn::parse_file(&file.to_token_stream().to_string())?;
		Ok(())
	}
}

#[derive(Debug)]
struct RsxVisitor {
	templates: Vec<TokenStream>,
	mac: syn::Ident,
}

impl Default for RsxVisitor {
	fn default() -> Self {
		Self {
			templates: Default::default(),
			mac: syn::parse_quote!(rsx),
		}
	}
}



impl<'a> Visit<'a> for RsxVisitor {
	fn visit_macro(&mut self, mac: &syn::Macro) {
		// println!("{:?}", i.tokens);
		if mac
			.path
			.segments
			.last()
			.map_or(false, |seg| seg.ident == self.mac)
		{
			let tokens = RstmlToRsxTemplate {
				exclude_errors: true,
				..Default::default()
			}
			.map_tokens(mac.tokens.clone());

			self.templates.push(tokens);
		}
	}
}

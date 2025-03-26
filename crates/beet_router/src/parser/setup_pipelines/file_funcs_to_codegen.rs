use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use sweet::prelude::*;
use syn::Block;
use syn::Expr;
use syn::File;
use syn::Signature;
use syn::Type;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFuncsToCodegen {
	/// The output codegen file location.
	pub output: WorkspacePathBuf,
	/// For internal use, these are the tokens to import beet functions,
	/// defaults to `use beet::prelude::*;`.
	pub use_beet_tokens: String,
	/// The function type to collect via [`IntoFileFunc`], defaults to [`DefaultFileFunc`].
	pub func_type: String,
	/// As `std::any::typename` resolves to a named crate, we need to alias the current
	/// crate to match any internal types, setting this option will add `use crate as pkg_name`
	/// to the top of the file.
	pub pkg_name: Option<String>,
}


impl Default for FileFuncsToCodegen {
	fn default() -> Self {
		Self {
			use_beet_tokens: "use beet::prelude::*;".into(),
			output: "src/codegen/mod.rs".into(),
			func_type: "DefaultFileFunc".into(),
			pkg_name: None,
		}
	}
}


impl RsxPipeline<Vec<FileFuncs>, Result<()>> for FileFuncsToCodegen {
	fn apply(self, funcs: Vec<FileFuncs>) -> Result<()> {
		self.build_and_write(funcs)
	}
}


impl FileFuncsToCodegen {
	pub fn build_and_write(&self, funcs: Vec<FileFuncs>) -> Result<()> {
		let output_tokens = self.build_output(funcs)?;
		let output_str = prettyplease::unparse(&output_tokens);

		let output_dir = self.output.into_canonical_unchecked()?;
		FsExt::write(&output_dir, &output_str)?;
		Ok(())
	}

	pub fn build_output(&self, funcs: Vec<FileFuncs>) -> Result<File> {
		let canonical_out = self.output.into_canonical_unchecked()?;
		let output_dir = canonical_out.parent().ok_or_else(|| {
			anyhow::anyhow!("Output path must have a parent directory")
		})?;
		let files = funcs
			.into_iter()
			.map(|file| self.build_file_output(&output_dir, file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		let use_beet: syn::Item = syn::parse_str(&self.use_beet_tokens)?;

		let func_type: Type = syn::parse_str(&self.func_type)?;
		let crate_alias = self.crate_alias()?;

		Ok(syn::parse_quote! {
			#use_beet
			#crate_alias
			pub fn collect() -> Vec<FileFunc<#func_type>> {
				vec![#(#files),*]
			}
		})
	}

	fn crate_alias(&self) -> Result<TokenStream> {
		let alias = if let Some(pkg_name) = &self.pkg_name {
			let pkg_name: Expr = syn::parse_str(pkg_name)?;
			quote! {
				#[allow(unused_imports)]
				use crate as #pkg_name;
			}
		} else {
			TokenStream::default()
		};
		Ok(alias)
	}

	pub fn build_file_output(
		&self,
		canonical_out_dir: &Path,
		file: FileFuncs,
	) -> Result<Vec<Block>> {
		let mod_path =
			PathExt::create_relative(canonical_out_dir, &file.canonical_path)?;
		let mod_path_str = mod_path.to_string_lossy();
		let local_path_str = file.local_path.to_string_lossy();

		let funcs = file
			.funcs
			.into_iter()
			.map(|sig| {
				self.build_func_output(&mod_path_str, &local_path_str, &sig)
			})
			.collect::<Result<Vec<_>>>()?;

		Ok(funcs)
	}

	pub fn build_func_output(
		&self,
		mod_path: &str,
		local_path: &str,
		sig: &Signature,
	) -> Result<syn::Block> {
		let ident = &sig.ident;
		let ident_str = ident.to_string();
		let func = syn::parse_quote! {{
		#[path=#mod_path]
			mod component;
			FileFunc::new(
				#local_path,
				#ident_str,
				component::#ident
			)
		}};
		Ok(func)
	}
}

#[cfg(test)]
mod test {
	// use crate::prelude::*;

	#[test]
	fn works() {
		// let file =
		// 	FileFuncsToCodegen{

		// 		..Default::default()
		// 	}
		// 		.build_output()
		// 		.unwrap();
		// let file = prettyplease::unparse(&file);
		// println!("{}", file);

		// let paths = config.build_strings().unwrap();
		// expect(paths.len()).to_be(2);
		// println!("{:#?}", paths);
	}
}

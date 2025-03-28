use anyhow::Result;
use beet_rsx::rsx::RsxPipelineTarget;
use proc_macro2::TokenStream;
use std::path::Path;
use sweet::prelude::*;
use syn::Expr;
use syn::Item;

/// Every codegen file is created via this struct. It contains
/// several utilities and standards that make the whole thing nicer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenFile {
	/// The output codegen file location.
	pub output: CanonicalPathBuf,
	/// For internal use, these are the tokens to import beet functions,
	/// defaults to `use beet::prelude::*;`.
	pub use_beet_tokens: String,
	/// As `std::any::typename` resolves to a named crate, we need to alias the current
	/// crate to match any internal types, setting this option will add `use crate as pkg_name`
	/// to the top of the file.
	pub pkg_name: Option<String>,
	// it'd be nice to use syn::Item but that isnt thread safe and
	// involves custom serde
	pub items: Vec<Item>,
}


impl Default for CodegenFile {
	fn default() -> Self {
		Self {
			use_beet_tokens: "use beet::prelude::*;".into(),
			output: WorkspacePathBuf::new("src/codegen/mod.rs")
				.into_canonical_unchecked(),
			pkg_name: None,
			items: Default::default(),
		}
	}
}


impl RsxPipelineTarget for CodegenFile {}

impl CodegenFile {
	/// Create a new [`CodegenFile`] with the most common options.
	pub fn new_workspace_rel(
		output: impl Into<WorkspacePathBuf>,
		pkg_name: impl Into<String>,
	) -> Self {
		let output = output.into().into_canonical_unchecked();
		Self {
			output,
			pkg_name: Some(pkg_name.into()),
			..Default::default()
		}
	}

	pub fn with_use_beet_tokens(
		mut self,
		use_beet_tokens: impl Into<String>,
	) -> Self {
		self.use_beet_tokens = use_beet_tokens.into();
		self
	}


	pub fn output_dir(&self) -> Result<&Path> {
		self.output.parent().ok_or_else(|| {
			anyhow::anyhow!("Output path must have a parent directory")
		})
	}

	pub fn add_item<T: Into<syn::Item>>(&mut self, item: T) {
		self.items.push(item.into());
	}

	pub fn build_output(&self) -> Result<syn::File> {
		let use_beet: syn::Item = syn::parse_str(&self.use_beet_tokens)?;
		let crate_alias = self.crate_alias()?;

		let items = &self.items;

		Ok(syn::parse_quote! {
			//! 🥁🥁🥁 This file has been auto generated by Beet.
			//! 🥁🥁🥁 Any changes will be overridden if the file is regenerated.
			#use_beet
			#crate_alias
			#(#items)*
		})
	}

	pub fn build_and_write(&self) -> Result<()> {
		let output_tokens = self.build_output()?;
		let output_str = prettyplease::unparse(&output_tokens);

		FsExt::write(&self.output, &output_str)?;
		Ok(())
	}
	fn crate_alias(&self) -> Result<syn::Item> {
		let alias = if let Some(pkg_name) = &self.pkg_name {
			let pkg_name: Expr = syn::parse_str(pkg_name)?;
			syn::parse_quote! {
				#[allow(unused_imports)]
				use crate as #pkg_name;
			}
		} else {
			syn::Item::Verbatim(TokenStream::default())
		};
		Ok(alias)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	use syn::ItemFn;

	#[test]
	fn works() {
		let mut file = CodegenFile::default();
		file.add_item::<ItemFn>(syn::parse_quote! {
			fn test() {}
		});
		let output = file.build_output().unwrap();
		let output_str = prettyplease::unparse(&output);
		expect(output_str).to_be("//! 🥁🥁🥁 This file has been auto generated by Beet.\n//! 🥁🥁🥁 Any changes will be overridden if the file is regenerated.\nuse beet::prelude::*;\nfn test() {}\n");
	}
}

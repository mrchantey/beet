use anyhow::Result;
use beet_rsx::rsx::RsxPipelineTarget;
use proc_macro2::TokenStream;
use quote::ToTokens;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use sweet::prelude::*;
use syn::Expr;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
	pub items: Vec<String>,
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
	pub fn output_dir(&self) -> Result<&Path> {
		self.output.parent().ok_or_else(|| {
			anyhow::anyhow!("Output path must have a parent directory")
		})
	}

	pub fn add_item<T: Into<syn::Item>>(&mut self, item: T) {
		self.items.push(item.into().to_token_stream().to_string());
	}

	pub fn build_output(&self) -> Result<syn::File> {
		let use_beet: syn::Item = syn::parse_str(&self.use_beet_tokens)?;
		let crate_alias = self.crate_alias()?;

		let items = self
			.items
			.iter()
			.map(|item| syn::parse_str(item))
			.collect::<syn::Result<Vec<syn::Item>>>()?;

		Ok(syn::parse_quote! {
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
		let ron =
			ron::ser::to_string_pretty(&file, Default::default()).unwrap();
		let file: CodegenFile = ron::de::from_str(&ron).unwrap();
		expect(file.items.len()).to_be(1);
	}
}

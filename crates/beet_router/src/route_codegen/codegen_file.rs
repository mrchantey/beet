//! Code generation file utilities.
//!
//! This module provides the [`CodegenFile`] component for representing
//! and building generated code files with proper imports and structure.

use beet_core::prelude::*;
use heck::ToSnakeCase;
use quote::ToTokens;
use syn::Expr;
use syn::Item;

/// Calls [`CodegenFile::build_and_write`] for every [`Changed<CodegenFile>`].
pub fn export_codegen(
	query: Populated<&CodegenFile, Changed<CodegenFile>>,
) -> bevy::prelude::Result {
	let num_files = query.iter().count();
	info!("Exporting {} codegen files...", num_files);
	for codegen_file in query.iter() {
		codegen_file.build_and_write()?;
	}
	Ok(())
}

/// Represents a generated code file with its configuration and contents.
///
/// Every codegen file is created via this struct, which provides utilities
/// for building well-formatted Rust code files with proper imports and
/// structure.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Default, Component)]
pub struct CodegenFile {
	/// The output codegen file location.
	output: AbsPathBuf,
	/// Package name alias for the current crate.
	///
	/// Since [`std::any::type_name`] resolves to a named crate (used with
	/// [`TemplateSerde`]), we need to alias the current crate to match any
	/// internal types. Setting this option adds `use crate as pkg_name;`
	/// to the top of the file.
	pkg_name: Option<SmolStr>,
	/// Imports to include at the top of the file.
	///
	/// These will not be erased when the file is regenerated.
	// Would be nice to store as Vec<Item> but bevy reflect doesn't support
	// custom serialization at this stage
	imports: Vec<String>,
	/// Root level items to be included in the file.
	///
	/// These are usually appended to as this struct is passed around.
	// Would be nice to store as Vec<Item> but bevy reflect doesn't support
	// custom serialization at this stage
	items: Vec<String>,
}

impl Default for CodegenFile {
	fn default() -> Self {
		Self {
			output: WsPathBuf::new("src/codegen/mod.rs").into_abs(),
			pkg_name: None,
			imports: default(),
			items: default(),
		}
		.with_import(syn::parse_quote!(
			#[allow(unused_imports)]
			use beet::prelude::*;
		))
		.with_import(syn::parse_quote!(
			#[allow(unused_imports)]
			use crate::prelude::*;
		))
	}
}

impl CodegenFile {
	/// Creates a new [`CodegenFile`] with the most common options.
	pub fn new(output: AbsPathBuf) -> Self {
		Self {
			output,
			..Default::default()
		}
	}

	/// Returns the output path for this codegen file.
	pub fn output(&self) -> &AbsPathBuf { &self.output }

	/// Returns the package name alias, if set.
	pub fn pkg_name(&self) -> Option<&SmolStr> { self.pkg_name.as_ref() }

	/// Returns the snake_case name of this codegen file.
	///
	/// If the file is a `mod.rs`, returns the parent directory name instead.
	pub fn name(&self) -> Result<String> {
		let stem = self
			.output
			.file_stem()
			.and_then(|stem| stem.to_str())
			.ok_or_else(|| {
				bevyhow!(
					"codegen output must have a file stem: {}",
					self.output
				)
			})?;
		if stem != "mod" {
			return Ok(stem.to_snake_case());
		}
		// mod files take their name from the parent directory
		let parent = self.output.parent().ok_or_else(|| {
			bevyhow!("mod files must have a parent: {}", self.output)
		})?;
		parent
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| {
				bevyhow!("mod files must have a named parent: {}", self.output)
			})?
			.to_snake_case()
			.xok()
	}

	/// Clones the metadata of this codegen file with a new output path.
	///
	/// The items list is cleared in the clone.
	pub fn clone_info(&self, output: AbsPathBuf) -> Self {
		Self {
			output,
			imports: self.imports.clone(),
			pkg_name: self.pkg_name.clone(),
			items: Vec::new(),
		}
	}

	/// Sets the package name alias for this codegen file.
	pub fn with_pkg_name(mut self, pkg_name: impl Into<SmolStr>) -> Self {
		self.pkg_name = Some(pkg_name.into());
		self
	}

	/// Adds an import item to this codegen file.
	pub fn with_import(mut self, item: Item) -> Self {
		self.imports.push(item.into_token_stream().to_string());
		self
	}

	/// Sets the imports for this codegen file.
	///
	/// This replaces any default or previously set imports.
	pub fn set_imports(mut self, items: Vec<Item>) -> Self {
		self.imports = items
			.iter()
			.map(|item| item.into_token_stream().to_string())
			.collect();
		self
	}

	/// Returns the output directory path.
	pub fn output_dir(&self) -> Result<AbsPathBuf> {
		self.output
			.parent()
			.ok_or_else(|| bevyhow!("Output path must have a parent directory"))
	}

	/// Clears all items from this codegen file.
	pub fn clear_items(&mut self) { self.items.clear(); }

	/// Adds an item to this codegen file.
	pub fn add_item<T: Into<Item>>(&mut self, item: T) {
		self.items.push(item.into().into_token_stream().to_string());
	}

	/// Converts the imports to syn tokens.
	fn imports_to_tokens(&self) -> Result<Vec<Item>, syn::Error> {
		self.imports
			.iter()
			.map(|s| syn::parse_str::<Item>(s))
			.collect::<Result<_, _>>()
	}

	/// Converts the items to syn tokens.
	fn items_to_tokens(&self) -> Result<Vec<Item>, syn::Error> {
		self.items
			.iter()
			.map(|s| syn::parse_str::<Item>(s))
			.collect::<Result<_, _>>()
	}

	/// Builds the output file as a syn [`File`](syn::File).
	pub fn build_output(&self) -> Result<syn::File> {
		let imports = self.imports_to_tokens()?;
		let crate_alias = self.crate_alias()?;

		let items = self.items_to_tokens()?;

		Ok(syn::parse_quote! {
			//! 🌱🌱🌱 This file has been auto generated by Beet.
			//! 🌱🌱🌱 Any changes will be overridden if the file is regenerated.
			#(#imports)*
			#crate_alias
			#(#items)*
		})
	}

	/// Builds the output file and writes it to the specified path if changed.
	pub fn build_and_write(&self) -> Result<()> {
		let output_tokens = self.build_output()?;
		// ideally we'd use rustfmt instead
		let output_str = prettyplease::unparse(&output_tokens);
		trace!("Exporting codegen file:\n{}", self.output.to_string_lossy());

		fs_ext::write_if_diff(&self.output, &output_str)?;
		Ok(())
	}

	/// Generates the crate alias item if a package name is set.
	fn crate_alias(&self) -> Result<Option<syn::Item>> {
		if let Some(pkg_name) = &self.pkg_name {
			let pkg_name: Expr = syn::parse_str(pkg_name.as_str())?;
			Ok(Some(syn::parse_quote! {
				#[allow(unused_imports)]
				use crate as #pkg_name;
			}))
		} else {
			Ok(None)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use quote::ToTokens;
	use syn::ItemFn;

	#[beet_core::test]
	fn works() {
		let mut file = CodegenFile::default();
		file.add_item::<ItemFn>(syn::parse_quote! {
			fn test() {}
		});
		(&file.build_output().unwrap().to_token_stream().to_string())
			.xpect_contains("fn test () { }");
	}
}

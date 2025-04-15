use crate::prelude::*;
use std::path::Path;
use sweet::prelude::*;
use syn::Item;



/// A group of [`FuncTokens`] for which a type has been
/// determined, and so is ready for codegen steps like creating
/// a `collect` function.
pub struct FuncTokensGroup {
	/// The return type of each [`FuncTokens::func`] block.
	pub func_type: syn::Type,
	/// A group of [`FuncTokens`] that are all the same type.
	pub funcs: Vec<FuncTokens>,
}


impl AsRef<FuncTokensGroup> for FuncTokensGroup {
	fn as_ref(&self) -> &FuncTokensGroup { self }
}


impl FuncTokensGroup {
	// this approach is cleaner than importing in each function,
	// and also rust-analyzer has an easier time resolving file level imports
	/// Return a list of `mod` imports for each [`FuncTokens::func`]
	/// that requires a module import.
	pub fn func_files_to_mod_imports(
		&self,
		canonical_out_dir: &Path,
	) -> Result<Vec<Item>> {
		self.funcs
			.iter()
			.filter_map(|func| match &func.mod_ident {
				Some(mod_ident) => Some((mod_ident, func)),
				None => None,
			})
			.map(|(mod_ident, func)| {
				let mod_path = PathExt::create_relative(
					canonical_out_dir,
					&func.canonical_path,
				)?;
				let mod_path_str = mod_path.to_string_lossy();
				let mod_import = syn::parse_quote! {
					#[path = #mod_path_str]
					pub mod #mod_ident;
				};
				Ok(mod_import)
			})
			.collect()
	}
}

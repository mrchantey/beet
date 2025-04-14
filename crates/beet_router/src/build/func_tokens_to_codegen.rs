use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::Path;
use sweet::prelude::*;
use syn::Item;


/// For a given [`FuncTokensGroup`], add the
/// `collect` function to the [`CodegenFile`] using its types
/// and importing required modules as directed.
#[derive(Debug, Default, Clone)]
pub struct FuncTokensGroupToCodegen {
	pub codegen_file: CodegenFile,
}


impl<T: AsRef<FuncTokensGroup>> Pipeline<T, Result<(T, CodegenFile)>>
	for FuncTokensGroupToCodegen
{
	fn apply(mut self, group: T) -> Result<(T, CodegenFile)> {
		let out_dir = self.codegen_file.output_dir()?;
		let collect_routes = self.routes_to_collect_func(group.as_ref())?;
		let mod_imports =
			self.func_files_to_mod_imports(out_dir, &group.as_ref().funcs)?;
		self.codegen_file.items.extend(mod_imports);
		self.codegen_file.items.push(collect_routes.into());

		Ok((group, self.codegen_file))
	}
}


impl FuncTokensGroupToCodegen {
	pub fn new(codegen_file: CodegenFile) -> Self {
		Self {
			codegen_file,
			..Default::default()
		}
	}

	fn routes_to_collect_func(
		&self,
		group: &FuncTokensGroup,
	) -> Result<syn::ItemFn> {
		let funcs = group.funcs.iter().map(|func| &func.func);
		let func_type = &group.func_type;

		Ok(syn::parse_quote! {
			/// Collect all functions from their files as defined in the [`AppConfig`]
			pub fn collect() -> Vec<#func_type> {
				vec![#(#funcs),*]
			}
		})
	}

	// this approach is cleaner than importing in each function,
	// and also rust-analyzer has an easier time resolving file level imports
	fn func_files_to_mod_imports(
		&self,
		canonical_out_dir: &Path,
		funcs: &Vec<FuncTokens>,
	) -> Result<Vec<Item>> {
		funcs
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;


	#[test]
	fn works() {
		let codegen_file = FileGroup::test_site_pages()
			.xpipe(FileGroupToFuncTokens::default())
			.unwrap()
			.xpipe(FuncTokensToRsxRoutesGroup::default())
			.xpipe(FuncTokensGroupToCodegen::default())
			.unwrap()
			.xmap(|(_, codegen_file)| codegen_file.build_output())
			.unwrap()
			.to_token_stream()
			.to_string();
		// coarse test, it compiles and outputs something
		expect(codegen_file.len()).to_be_greater_than(500);
		// ensure no absolute paths
		// println!("{}", codegen_file);
		expect(codegen_file).not().to_contain("/home/");
	}

	// #[test]
	// fn custom() {
	// 	let codegen_file = vec![FuncTokens::simple(
	// 		"docs/index.rs",
	// 		syn::parse_quote! {|| rsx! { "hello world" }},
	// 	)]
	// 	.xpipe(FuncTokensGroupToCodegen::default().with_map_tokens(
	// 		syn::parse_quote!(String),
	// 		|func| {
	// 			let block = &func.func;
	// 			syn::parse_quote! {{
	// 				#block.to_string()
	// 			}}
	// 		},
	// 	))
	// 	.unwrap()
	// 	.xmap(|(_, codegen_file)| codegen_file.build_output())
	// 	.unwrap()
	// 	.to_token_stream()
	// 	.to_string();
	// 	expect(&codegen_file).to_contain("pub fn collect () -> Vec < String > { vec ! [{ | | rsx ! { \"hello world\" } . to_string () }] }");
	// }
}

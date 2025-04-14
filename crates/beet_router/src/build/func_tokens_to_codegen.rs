use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::Path;
use sweet::prelude::*;
use syn::Block;
use syn::Item;

// #[derive(Clone)]
pub struct FuncTokensToCodegen {
	/// The return type of the `collect` function, this must
	/// match the output of the [`FuncTokensToCodegen::map_func_tokens`]
	pub func_type: syn::Type,
	pub codegen_file: CodegenFile,
	/// Map the func tokens, this must return the [`FuncTokensToCodegen::func_type`]
	pub map_func_tokens: Box<dyn Fn(&FuncTokens) -> Block>,
}

impl Default for FuncTokensToCodegen {
	fn default() -> Self {
		Self {
			func_type: syn::parse_quote!(RouteFunc<DefaultRouteFunc>),
			codegen_file: CodegenFile::default(),
			map_func_tokens: Box::new(func_tokens_to_route_funcs),
		}
	}
}

/// Create the tokens for a [`RouteFunc`], usually the final
/// transformation before codegen.
/// We need to do this at a late stage to allow transformations
/// to be applied.
fn func_tokens_to_route_funcs(func_tokens: &FuncTokens) -> Block {
	let method = func_tokens.route_info.method.to_string();
	let route_path = func_tokens.route_info.path.to_string_lossy();
	let block = &func_tokens.func;
	// TODO frontmatter
	syn::parse_quote! {{
	RouteFunc::new(
		#method,
		#route_path,
		#block
	)}}
}


impl<T: AsRef<Vec<FuncTokens>>> Pipeline<T, Result<(T, CodegenFile)>>
	for FuncTokensToCodegen
{
	fn apply(self, func_tokens: T) -> Result<(T, CodegenFile)> {
		let out_dir = self.codegen_file.output_dir()?;
		let collect_routes =
			self.routes_to_collect_func(func_tokens.as_ref())?;
		let mod_imports =
			self.func_files_to_mod_imports(out_dir, func_tokens.as_ref())?;
		let mut codegen_file = self.codegen_file;
		codegen_file.items.extend(mod_imports);
		codegen_file.items.push(collect_routes.into());

		Ok((func_tokens, codegen_file))
	}
}


impl FuncTokensToCodegen {
	pub fn new(codegen_file: CodegenFile) -> Self {
		Self {
			codegen_file,
			..Default::default()
		}
	}

	pub fn with_map_tokens(
		self,
		func_type: syn::Type,
		map_func_tokens: impl Fn(&FuncTokens) -> Block + 'static,
	) -> Self {
		Self {
			func_type,
			map_func_tokens: Box::new(map_func_tokens),
			..self
		}
	}

	fn routes_to_collect_func(
		&self,
		funcs: &Vec<FuncTokens>,
	) -> Result<syn::ItemFn> {
		let blocks = funcs.iter().map(|func| (self.map_func_tokens)(func));
		let func_type = &self.func_type;

		Ok(syn::parse_quote! {
			/// Collect all functions from their files as defined in the [`AppConfig`]
			pub fn collect() -> Vec<#func_type> {
				vec![#(#blocks),*]
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
		let codegen_file = FileGroup::test_site_routes()
			.xpipe(FileGroupToFuncTokens::default())
			.unwrap()
			.xpipe(FuncTokensToCodegen::default())
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
}

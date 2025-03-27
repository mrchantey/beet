use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use sweet::prelude::*;
use syn::Expr;
use syn::Ident;
use syn::ItemMod;
use syn::Signature;
use syn::Type;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFuncsToCodegen {
	pub func_type: String,
}


impl Default for FileFuncsToCodegen {
	fn default() -> Self {
		Self {
			func_type: "DefaultFileFunc".into(),
		}
	}
}


impl RsxPipeline<(Vec<FileFuncs>, CodegenFile), Result<CodegenFile>>
	for FileFuncsToCodegen
{
	fn apply(
		self,
		(funcs, mut file): (Vec<FileFuncs>, CodegenFile),
	) -> Result<CodegenFile> {
		self.append_collect_func(&mut file, funcs)?;
		Ok(file)
	}
}


impl FileFuncsToCodegen {
	pub fn append_collect_func(
		&self,
		file: &mut CodegenFile,
		funcs: Vec<FileFuncs>,
	) -> Result<()> {
		let collect_func = self.build_item_fn(&funcs)?;
		let mod_imports =
			self.file_funcs_to_mod_imports(file.output_dir()?, &funcs)?;
		for item in mod_imports.into_iter() {
			file.add_item(item);
		}
		file.add_item(collect_func);
		Ok(())
	}

	fn build_item_fn(&self, funcs: &Vec<FileFuncs>) -> Result<syn::ItemFn> {
		let files = funcs
			.iter()
			.enumerate()
			.map(|(index, file)| self.file_funcs_to_collect(index, file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		let func_type: Type = syn::parse_str(&self.func_type)?;

		Ok(syn::parse_quote! {
			pub fn collect() -> Vec<FileFunc<#func_type>> {
				vec![#(#files),*]
			}
		})
	}

	// this approach is cleaner than importing in each collect function,
	// and also rust-analyzer has an easier time resolving file level imports
	fn file_funcs_to_mod_imports(
		&self,
		canonical_out_dir: &Path,
		funcs: &Vec<FileFuncs>,
	) -> Result<Vec<ItemMod>> {
		funcs
			.iter()
			.enumerate()
			.map(|(index, file)| {
				let mod_path = PathExt::create_relative(
					canonical_out_dir,
					&file.canonical_path,
				)?;
				let mod_path_str = mod_path.to_string_lossy();
				let mod_ident = Self::index_to_mod_ident(index);
				let mod_import = syn::parse_quote! {
					#[path = #mod_path_str]
					mod #mod_ident;
				};
				Ok(mod_import)
			})
			.collect()
	}

	pub fn file_funcs_to_collect(
		&self,
		index: usize,
		file: &FileFuncs,
	) -> Result<Vec<Expr>> {
		let local_path_str = file.local_path.to_string_lossy();
		let mod_ident = Self::index_to_mod_ident(index);
		file.funcs
			.iter()
			.map(|sig| {
				self.file_func_to_collect(&mod_ident, &local_path_str, &sig)
			})
			.collect()
	}

	pub fn file_func_to_collect(
		&self,
		mod_ident: &Ident,
		local_path: &str,
		sig: &Signature,
	) -> Result<syn::Expr> {
		let ident = &sig.ident;
		let ident_str = ident.to_string();
		let func = syn::parse_quote! {
			FileFunc::new(
				#local_path,
				#ident_str,
				#mod_ident::#ident
			)
		};
		Ok(func)
	}
	fn index_to_mod_ident(index: usize) -> Ident {
		Ident::new(&format!("file{}", index), proc_macro2::Span::call_site())
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
		let codegen_file = CodegenFile::default();
		let codegen_file = FileGroup::test_site_routes()
			.pipe(FileGroupToFuncs::default())
			.unwrap()
			.pipe_with(codegen_file, FileFuncsToCodegen::default())
			.unwrap()
			.build_output()
			.unwrap()
			.to_token_stream()
			.to_string();
		// coarse test, it compiles and outputs something
		expect(codegen_file.len()).to_be_greater_than(500);
		// ensure no absolute paths
		println!("{}", codegen_file);
		expect(codegen_file).not().to_contain("/home/");
	}
}

use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use sweet::prelude::*;
use syn::Block;
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
		let collect_func = self.build_item_fn(file.output_dir()?, funcs)?;
		file.add_item(collect_func);
		Ok(())
	}

	fn build_item_fn(
		&self,
		out_dir: &Path,
		funcs: Vec<FileFuncs>,
	) -> Result<syn::ItemFn> {
		let files = funcs
			.into_iter()
			.map(|file| self.file_funcs_to_blocks(&out_dir, file))
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

	pub fn file_funcs_to_blocks(
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
				self.file_func_to_block(&mod_path_str, &local_path_str, &sig)
			})
			.collect::<Result<Vec<_>>>()?;

		Ok(funcs)
	}

	pub fn file_func_to_block(
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
		// println!("{}", codegen_file);
	}
}

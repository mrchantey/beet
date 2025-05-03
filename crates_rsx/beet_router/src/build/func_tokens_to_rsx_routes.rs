use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;




#[derive(Default)]
pub struct FuncTokensToRsxRoutes {
	pub codegen_file: CodegenFile,
}


impl Pipeline<FuncTokensGroup, Result<(FuncTokensGroup, CodegenFile)>>
	for FuncTokensToRsxRoutes
{
	fn apply(
		mut self,
		group: FuncTokensGroup,
	) -> Result<(FuncTokensGroup, CodegenFile)> {
		let mod_imports = group.as_ref().item_mods(&self.codegen_file)?;

		let collect_routes = group.collect_func(
			&syn::parse_quote!(RouteFunc<RsxRouteFunc>),
			|func| {
				let func_path = &func.func_path();
				let route_info = &func.route_info;
				syn::parse_quote! {
					RouteFunc::new(
						#route_info,
						#func_path
					)
				}
			},
		);

		self.codegen_file.items.extend(mod_imports);
		self.codegen_file.items.push(collect_routes.into());

		Ok((group, self.codegen_file))
	}
}


impl FuncTokensToRsxRoutes {
	pub fn new(codegen_file: CodegenFile) -> Self {
		Self {
			codegen_file,
			..Default::default()
		}
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
			.xpipe(FuncTokensToRsxRoutes::default())
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

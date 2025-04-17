use anyhow::Result;
use beet_router::prelude::*;
use sweet::prelude::*;

/// Maps the [`FuncTokens::func`] blocks so that it returns a
/// [`RouteFunc<RegisterAxumRoute>`]
pub struct FuncTokensToAxumRoutes {
	pub state_type: syn::Type,
	pub codegen_file: CodegenFile,
}
impl Default for FuncTokensToAxumRoutes {
	fn default() -> Self {
		FuncTokensToAxumRoutes {
			state_type: syn::parse_quote!(()),
			codegen_file: CodegenFile::default(),
		}
	}
}

impl FuncTokensToAxumRoutes {
	pub fn new(state_type: syn::Type) -> Self {
		FuncTokensToAxumRoutes {
			state_type,
			..Default::default()
		}
	}
}

impl Pipeline<FuncTokensGroup, Result<(FuncTokensGroup, CodegenFile)>>
	for FuncTokensToAxumRoutes
{
	fn apply(
		mut self,
		group: FuncTokensGroup,
	) -> Result<(FuncTokensGroup, CodegenFile)> {
		let mod_imports = group.as_ref().item_mods(&self.codegen_file)?;
		let state_type = self.state_type;
		let collect_routes = group.collect_func(
			&syn::parse_quote!(RouteFunc<RegisterAxumRoute<#state_type>>),
			|func| {
				let func_path = &func.func_path();
				let route_info = &func.route_info;
				syn::parse_quote! {
					RouteFunc::new(
						#route_info,
						#func_path.into_register_axum_route(&#route_info)
					)
				}
			},
		);

		self.codegen_file.items.extend(mod_imports);
		self.codegen_file.items.push(collect_routes.into());

		Ok((group, self.codegen_file))
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		FuncTokens::simple_get("/foo")
			.xinto::<FuncTokensGroup>()
			.xpipe(FuncTokensToAxumRoutes::default())
			.unwrap()
			.1
			.build_output()
			.unwrap()
			.xmap(|f| f.to_token_stream().to_string())
			.xmap(expect)
			.to_contain(". into_register_axum_route ()");
	}
}

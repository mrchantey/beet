// use crate::prelude::*;
use beet_router::prelude::*;
use sweet::prelude::*;

/// Maps the [`FuncTokens::func`] blocks so that it returns a
/// [`RouteFunc<RegisterAxumRoute>`]
pub struct FuncTokensToAxumRoutesGroup {
	pub state_type: syn::Type,
}
impl Default for FuncTokensToAxumRoutesGroup {
	fn default() -> Self {
		FuncTokensToAxumRoutesGroup {
			state_type: syn::parse_quote!(()),
		}
	}
}

impl Pipeline<Vec<FuncTokens>, FuncTokensGroup>
	for FuncTokensToAxumRoutesGroup
{
	fn apply(self, funcs: Vec<FuncTokens>) -> FuncTokensGroup {
		let state_type = self.state_type;
		FuncTokensGroup {
			func_type: syn::parse_quote!(RouteFunc<RegisterAxumRoute<#state_type>>),
			funcs: funcs.xmap_each(|mut func| {
				let block = &func.func;
				let route_info = &func.route_info;
				func.func = syn::parse_quote! {{
					RouteFunc::new(
						#route_info,
						#block.into_register_axum_route()
					)
				}};
				func
			}),
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::build::FuncTokens;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let tokens =
			vec![FuncTokens::simple("/foo", syn::parse_quote! {|| Ok(())})]
				.xpipe(FuncTokensToAxumRoutesGroup::default());
		expect(tokens.funcs[0].func.to_token_stream().to_string())
			.to_contain(". into_register_axum_route ()");
	}
}

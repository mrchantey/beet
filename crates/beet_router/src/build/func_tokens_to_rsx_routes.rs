use crate::prelude::*;
use sweet::prelude::*;





/// Maps the [`FuncTokens::func`] blocks so that it returns a
/// [`RouteFunc<RsxRouteFunc>`]
#[derive(Default)]
pub struct FuncTokensToRsxRoutesGroup;


impl Pipeline<Vec<FuncTokens>, FuncTokensGroup> for FuncTokensToRsxRoutesGroup {
	fn apply(self, funcs: Vec<FuncTokens>) -> FuncTokensGroup {
		FuncTokensGroup {
			func_type: syn::parse_quote!(RouteFunc<RsxRouteFunc>),
			funcs: funcs.xmap_each(|mut func| {
				let block = &func.func;
				let route_info = &func.route_info;
				func.func = syn::parse_quote! {{
					RouteFunc::new(
						#route_info,
						#block
					)
				}};
				func
			}),
		}
	}
}

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
				let method = func.route_info.method.to_string();
				let route_path = func.route_info.path.to_string_lossy();
				func.func = syn::parse_quote! {{
					RouteFunc::new(
						#method,
						#route_path,
						#block
					)
				}};
				func
			}),
		}
	}
}

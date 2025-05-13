use beet_router::types::RouteInfo;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use sweet::prelude::*;
use syn::Ident;



pub fn route_info_to_tokens(route_info: &RouteInfo) -> TokenStream {
	let path = &route_info.path.to_string_lossy();
	let method = http_method_to_tokens(&route_info.method);
	quote::quote! {
		RouteInfo::new(#path, #method)
	}
}

pub fn http_method_to_tokens(method: &HttpMethod) -> TokenStream {
	let method = Ident::new(&method.to_string(), Span::call_site());
	quote::quote! {
		HttpMethod::#method
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::types::RouteInfo;
	use sweet::prelude::*;

	#[test]
	fn to_tokens() {
		expect(
			route_info_to_tokens(&RouteInfo::new("/", HttpMethod::Get))
				.to_string(),
		)
		.to_be(
			quote::quote! {
				RouteInfo::new("/", HttpMethod::Get)
			}
			.to_string(),
		);
	}
}

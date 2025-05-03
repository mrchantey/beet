use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::RsxNode;
use std::path::PathBuf;
use sweet::prelude::*;

pub trait RoutesToRsx {
	async fn routes_to_rsx(&mut self) -> Result<Vec<(RouteInfo, RsxNode)>>;
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteInfo {
	/// the url path
	pub path: RoutePath,
	/// the http method
	pub method: HttpMethod,
}

impl RouteInfo {
	/// Whether the [`HttpMethod`] is of the type that expects a body
	pub fn has_body(&self) -> bool { self.method.has_body() }
}

#[cfg(feature = "parser")]
impl quote::ToTokens for RouteInfo {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let path = &self.path.to_string_lossy();
		let method = http_method_to_tokens(&self.method);
		tokens.extend(quote::quote! {
					RouteInfo::new(#path, #method)
		});
	}
}

fn http_method_to_tokens(method: &HttpMethod) -> proc_macro2::TokenStream {
	use proc_macro2::Span;
	use syn::Ident;
	let method = Ident::new(&method.to_string(), Span::call_site());
	quote::quote! {
		HttpMethod::#method
	}
}
impl RouteInfo {
	/// the method used by `beet_router`
	pub fn new(path: impl Into<PathBuf>, method: HttpMethod) -> Self {
		Self {
			method,
			path: RoutePath::new(path),
		}
	}
}

impl std::fmt::Display for RouteInfo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} {}", self.method, self.path)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		use quote::ToTokens;

		expect(
			RouteInfo::new("/", HttpMethod::Get)
				.to_token_stream()
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

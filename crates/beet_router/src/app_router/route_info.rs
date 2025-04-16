use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::RsxNode;
use http::Method;
use std::path::PathBuf;
use std::str::FromStr;

pub trait RoutesToRsx {
	async fn routes_to_rsx(&mut self) -> Result<Vec<(RouteInfo, RsxNode)>>;
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteInfo {
	/// the url path
	pub path: RoutePath,
	/// the http method
	#[cfg_attr(
		feature = "serde",
		serde(
			serialize_with = "serialize_method",
			deserialize_with = "deserialize_method"
		)
	)]
	pub method: Method,
}

impl RouteInfo {
	/// Whether the [`Method`] is of the type that
	/// expects a body
	pub fn method_has_body(&self) -> bool {
		self.method == Method::POST
			|| self.method == Method::PUT
			|| self.method == Method::PATCH
	}
}

#[cfg(feature = "parser")]
impl quote::ToTokens for RouteInfo {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		use proc_macro2::Span;
		use syn::Ident;
		let path = &self.path.to_string_lossy();
		let method =
			Ident::new(&self.method.as_str().to_uppercase(), Span::call_site());
		tokens.extend(quote::quote! {
					RouteInfo::new(#path,beet::exports::http::Method::#method)
		});
	}
}


#[cfg(feature = "serde")]
fn serialize_method<S>(
	method: &Method,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	serializer.serialize_str(method.as_str())
}

#[cfg(feature = "serde")]
fn deserialize_method<'de, D>(deserializer: D) -> Result<Method, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;
	let s = String::deserialize(deserializer)?;
	Method::from_str(&s).map_err(serde::de::Error::custom)
}


impl RouteInfo {
	/// the method used by `beet_router`
	pub fn new(path: impl Into<PathBuf>, method: Method) -> Self {
		Self {
			method,
			path: RoutePath::new(path),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		use http::Method;
		use quote::ToTokens;

		expect(
			RouteInfo::new("/", Method::GET)
				.to_token_stream()
				.to_string(),
		)
		.to_be(
			quote::quote! {
				RouteInfo::new("/", beet::exports::http::Method::GET)
			}
			.to_string(),
		);
	}
}

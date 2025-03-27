use anyhow::Result;
use beet_rsx::rsx::RsxPipelineTarget;
use beet_rsx::rsx::RsxRoot;
use http::Method;
use std::path::PathBuf;
use std::str::FromStr;


pub trait RoutesToRsx {
	async fn routes_to_rsx(&mut self) -> Result<Vec<(RouteInfo, RsxRoot)>>;
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteInfo {
	/// the url path
	pub path: PathBuf,
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

#[cfg(feature = "parser")]
impl quote::ToTokens for RouteInfo {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let path = &self.path.to_string_lossy();
		let method = &self.method.to_string();
		tokens.extend(quote::quote! {
					RouteInfo::new(#path, #method)
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
	pub fn new(path: impl Into<PathBuf>, method: &str) -> Self {
		Self {
			path: path.into(),
			method: Method::from_str(method).unwrap(),
		}
	}
}

impl RsxPipelineTarget for RouteInfo {}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		use quote::ToTokens;

		expect(RouteInfo::new("/", "GET").to_token_stream().to_string()).to_be(
			quote::quote! {
				RouteInfo::new("/", "GET")
			}
			.to_string(),
		);
	}
}

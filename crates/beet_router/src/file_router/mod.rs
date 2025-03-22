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
	pub fn new(path: &str, method: &str) -> Self {
		Self {
			path: PathBuf::from(path),
			method: Method::from_str(method).unwrap(),
		}
	}
}


impl RsxPipelineTarget for RouteInfo {}

pub struct RouteTree<T> {
	/// The path to the auto generated mod file for this tree,
	/// usually something like `src/routes/mod.rs`
	pub mod_path: PathBuf,
	pub routes: Vec<(RouteInfo, T)>,
	pub children: Vec<RouteTree<T>>,
}

impl<T> RouteTree<T> {
	pub fn add_route<M, R: IntoRoute<T, M>>(
		mut self,
		(info, route): (RouteInfo, R),
	) -> Self {
		self.routes.push((info, route.into_route()));
		self
	}
	pub fn flatten(self) -> Vec<(RouteInfo, T)> {
		let mut routes = self.routes;
		for child in self.children {
			routes.extend(child.flatten());
		}
		routes
	}
}

/// Routes usually need a level of trait indirection to
/// allow for multiple types of routes to be added to the same tree
pub trait IntoRoute<T, M>: 'static {
	fn into_route(&self) -> T;
}


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

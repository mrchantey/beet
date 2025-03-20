#[cfg(all(not(target_arch = "wasm32"), feature = "parser"))]
pub mod static_file_router;
#[cfg(all(not(target_arch = "wasm32"), feature = "parser"))]
pub use static_file_router::*;

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

#[derive(Default)]
pub struct RouteTree<T> {
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
}

/// Routes usually need a level of trait indirection to
/// allow for multiple types of routes to be added to the same tree
pub trait IntoRoute<T, M>: 'static {
	fn into_route(&self) -> T;
}

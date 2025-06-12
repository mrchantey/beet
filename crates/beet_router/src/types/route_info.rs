use crate::prelude::*;
use http::request::Parts;
use std::path::PathBuf;

// pub trait RoutesToRsx {
// 	async fn routes_to_rsx(&mut self) -> Result<Vec<(RouteInfo, WebNode)>>;
// }


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

impl RouteInfo {
	/// the method used by `beet_router`
	pub fn new(
		path: impl Into<PathBuf>,
		method: impl Into<HttpMethod>,
	) -> Self {
		Self {
			method: method.into(),
			path: RoutePath::new(path),
		}
	}
	pub fn from_parts(parts: &Parts) -> Self {
		Self::new(parts.uri.path(), &parts.method)
	}
	pub fn get(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Get)
	}
}

impl std::fmt::Display for RouteInfo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} {}", self.method, self.path)
	}
}

impl From<Parts> for RouteInfo {
	fn from(parts: Parts) -> Self {
		let path = parts.uri.path().to_string();
		let method = HttpMethod::from(parts.method);
		Self::new(path, method)
	}
}

impl Into<RouteInfo> for &str {
	fn into(self) -> RouteInfo { RouteInfo::get(self) }
}

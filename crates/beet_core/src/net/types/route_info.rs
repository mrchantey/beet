#[cfg(feature = "tokens")]
use crate::as_beet::*;
use crate::prelude::*;
use beet_common_macros::ToTokens;
use http::request::Parts;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
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
	pub fn post(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Post)
	}
	pub fn put(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Put)
	}
	pub fn delete(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Delete)
	}
	pub fn patch(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Patch)
	}
	pub fn head(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Head)
	}
	pub fn options(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Options)
	}
	pub fn trace(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Trace)
	}
	pub fn connect(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Connect)
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

use crate::prelude::*;
#[cfg(feature = "tokens")]
use beet_core::prelude::ToTokens;
use std::path::PathBuf;

/// Information about a route, containing the path and HTTP method.
///
/// This is a lightweight type used for route registration and matching.
/// It can be converted to/from [`RequestParts`] and [`Request`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RouteInfo {
	/// The URL path
	pub path: RoutePath,
	/// The HTTP method
	pub method: HttpMethod,
}

impl RouteInfo {
	/// Whether the [`HttpMethod`] is of the type that expects a body
	pub fn has_body(&self) -> bool { self.method.has_body() }
}

impl RouteInfo {
	/// Creates a new RouteInfo with the given path and method
	pub fn new(
		path: impl Into<PathBuf>,
		method: impl Into<HttpMethod>,
	) -> Self {
		Self {
			method: method.into(),
			path: RoutePath::new(path),
		}
	}

	/// Converts this `RouteInfo` into an `http::Request` with the given body
	pub fn into_request<T>(self, body: T) -> http::Result<http::Request<T>> {
		http::Request::builder()
			.method(self.method)
			.uri(self.path)
			.body(body)
	}

	/// Creates a RouteInfo from RequestParts
	pub fn from_parts(parts: &RequestParts) -> Self {
		Self::new(parts.path_string(), *parts.method())
	}

	/// Creates a RouteInfo from http::request::Parts
	pub fn from_http_parts(parts: &http::request::Parts) -> Self {
		Self::new(parts.uri.path(), &parts.method)
	}

	/// Creates a GET route for the given path
	pub fn get(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Get)
	}

	/// Creates a POST route for the given path
	pub fn post(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Post)
	}

	/// Creates a PUT route for the given path
	pub fn put(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Put)
	}

	/// Creates a DELETE route for the given path
	pub fn delete(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Delete)
	}

	/// Creates a PATCH route for the given path
	pub fn patch(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Patch)
	}

	/// Creates a HEAD route for the given path
	pub fn head(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Head)
	}

	/// Creates an OPTIONS route for the given path
	pub fn options(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Options)
	}

	/// Creates a TRACE route for the given path
	pub fn trace(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Trace)
	}

	/// Creates a CONNECT route for the given path
	pub fn connect(path: impl Into<PathBuf>) -> Self {
		Self::new(path, HttpMethod::Connect)
	}
}

impl std::fmt::Display for RouteInfo {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{} {}", self.method, self.path)
	}
}

impl From<RequestParts> for RouteInfo {
	fn from(parts: RequestParts) -> Self { Self::from_parts(&parts) }
}

impl From<&RequestParts> for RouteInfo {
	fn from(parts: &RequestParts) -> Self { Self::from_parts(parts) }
}

impl From<http::request::Parts> for RouteInfo {
	fn from(parts: http::request::Parts) -> Self {
		Self::from_http_parts(&parts)
	}
}

impl From<&http::request::Parts> for RouteInfo {
	fn from(parts: &http::request::Parts) -> Self {
		Self::from_http_parts(parts)
	}
}

impl Into<RouteInfo> for &str {
	fn into(self) -> RouteInfo { RouteInfo::get(self) }
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn route_info_new() {
		let info = RouteInfo::new("/api/users", HttpMethod::Get);
		info.path.to_string().xpect_eq("/api/users");
		info.method.xpect_eq(HttpMethod::Get);
	}

	#[test]
	fn route_info_display() {
		let info = RouteInfo::post("/api/users");
		format!("{}", info).xpect_eq("Post /api/users");
	}

	#[test]
	fn route_info_from_request_parts() {
		let parts = RequestParts::post("/api/users");
		let info = RouteInfo::from(&parts);

		info.method.xpect_eq(HttpMethod::Post);
		info.path.to_string().xpect_eq("/api/users");
	}

	#[test]
	fn route_info_from_str() {
		let info: RouteInfo = "/api/users".into();
		info.method.xpect_eq(HttpMethod::Get);
		info.path.to_string().xpect_eq("/api/users");
	}

	#[test]
	fn route_info_has_body() {
		RouteInfo::get("/test").has_body().xpect_false();
		RouteInfo::post("/test").has_body().xpect_true();
		RouteInfo::put("/test").has_body().xpect_true();
		RouteInfo::patch("/test").has_body().xpect_true();
		RouteInfo::delete("/test").has_body().xpect_false();
	}
}

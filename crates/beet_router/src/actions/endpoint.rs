use crate::prelude::*;
use beet_core::prelude::*;


/// A route endpoint that matches requests by method and path.
///
/// Endpoints are terminal actions that handle requests when the method and path
/// match exactly. Unlike middleware entities that may run for multiple child paths,
/// endpoints require an exact path match (no trailing segments).
///
/// # Construction
///
/// Use [`Endpoint::get`], [`Endpoint::post`], etc. to create endpoints:
///
/// ```ignore
/// # use beet_router::prelude::*;
/// Endpoint::get("/users/:id").body("User details")
/// ```
///
/// See [`EndpointBuilder`] for the full builder API.
#[derive(Debug, Clone, Component, PartialEq, Eq)]
pub struct Endpoint {
	/// An optional description for this endpoint.
	pub description: Option<String>,
	/// Query and path parameter patterns for this endpoint.
	pub params: ParamsPattern,
	/// The full [`PathPattern`] for this endpoint.
	pub path: PathPattern,
	/// The HTTP method to match, or `None` for any method.
	pub method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any.
	pub cache_strategy: Option<CacheStrategy>,
	/// Whether this endpoint is canonical (registered in [`EndpointTree`]).
	///
	/// Non-canonical endpoints are fallbacks that won't conflict with canonical routes.
	/// Defaults to `true`.
	pub is_canonical: bool,
	/// Metadata describing the expected request body.
	pub request_body: BodyType,
	/// Metadata describing the response body.
	pub response_body: BodyType,
}


impl Endpoint {
	#[cfg(test)]
	pub(crate) fn new(
		path: PathPattern,
		params: ParamsPattern,
		method: Option<HttpMethod>,
		cache_strategy: Option<CacheStrategy>,
		is_canonical: bool,
	) -> Self {
		Self {
			path,
			params,
			method,
			cache_strategy,
			is_canonical,
			description: None,
			request_body: BodyType::none(),
			response_body: BodyType::none(),
		}
	}

	/// Returns the endpoint description, if set.
	pub fn description(&self) -> Option<&str> { self.description.as_deref() }
	/// Returns the path pattern for this endpoint.
	pub fn path(&self) -> &PathPattern { &self.path }
	/// Returns the parameter patterns for this endpoint.
	pub fn params(&self) -> &ParamsPattern { &self.params }
	/// Returns the HTTP method this endpoint matches, or `None` for any method.
	pub fn method(&self) -> Option<HttpMethod> { self.method }
	/// Returns the cache strategy for this endpoint.
	pub fn cache_strategy(&self) -> Option<CacheStrategy> {
		self.cache_strategy
	}
	/// Returns whether this endpoint is canonical.
	pub fn is_canonical(&self) -> bool { self.is_canonical }
	/// The request body metadata
	pub fn request_body(&self) -> &BodyType { &self.request_body }
	/// The response body metadata
	pub fn response_body(&self) -> &BodyType { &self.response_body }

	/// Determines if this endpoint is a static GET endpoint
	pub fn is_static_get(&self) -> bool {
		self.path.is_static()
			&& self.method.map(|m| m == HttpMethod::Get).unwrap_or(true)
			&& self
				.cache_strategy
				.map(|s| s == CacheStrategy::Static)
				.unwrap_or(false)
	}
	/// Determines if this endpoint is a static GET endpoint returning HTML
	pub fn is_static_get_html(&self) -> bool {
		self.is_static_get() && self.response_body.is_html()
	}
}

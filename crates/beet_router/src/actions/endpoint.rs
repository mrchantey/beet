use crate::prelude::*;
use beet_core::prelude::*;


/// Endpoints are actions that will only run if the method and path are an
/// exact match. There should only be one of these per route match,
/// unlike non-endpoint entities that behave as middleware.
///
/// Usually this is not added directly, instead via the [`Endpoint::build`] constructor.
/// Endpoints should only run if there are no trailing path segments,
/// unlike middleware which may run for multiple child paths. See [`check_exact_path`]
#[derive(Debug, Clone, Component, PartialEq, Eq)]
pub struct Endpoint {
	/// An optional description for this endpoint
	pub description: Option<String>,
	pub params: ParamsPattern,
	/// The full [`PathPattern`] for this endpoint
	pub path: PathPattern,
	/// The method to match, or None for any method.
	pub method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any
	pub cache_strategy: Option<CacheStrategy>,
	/// Canonical endpoints are registered in the EndpointTree. Non-canonical endpoints
	/// are fallbacks that won't conflict with canonical routes. Defaults to `true`.
	pub is_canonical: bool,
	/// Metadata describing the expected request body
	pub request_body: BodyType,
	/// Metadata describing the response body
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

	pub fn description(&self) -> Option<&str> { self.description.as_deref() }
	pub fn path(&self) -> &PathPattern { &self.path }
	pub fn params(&self) -> &ParamsPattern { &self.params }
	pub fn method(&self) -> Option<HttpMethod> { self.method }
	pub fn cache_strategy(&self) -> Option<CacheStrategy> {
		self.cache_strategy
	}
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

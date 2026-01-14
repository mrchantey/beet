//! Generic request type for routing.
//!
//! The [`Request`] type abstracts over different transport mechanisms,
//! allowing the same routing infrastructure to handle HTTP requests,
//! CLI commands, and REPL input.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! // Create an HTTP-style request
//! let request = Request::get("/api/users?limit=10");
//! assert_eq!(request.path(), &["api", "users"]);
//! assert_eq!(request.get_param("limit"), Some(&"10".to_string()));
//!
//! // Create from CLI args
//! let cli = CliArgs::parse("users list --limit 10");
//! let request = Request::from(cli);
//! assert_eq!(request.path(), &["users", "list"]);
//! ```

#[cfg(feature = "http")]
use super::http_ext;
use crate::prelude::*;
use bytes::Bytes;
#[cfg(feature = "http")]
use http::header::IntoHeaderName;

/// A generalized request type that can represent HTTP requests, CLI commands,
/// or other request-response patterns.
///
/// This is a [`Component`] that is added to route entities before processing.
/// It contains both the request metadata ([`RequestParts`]) and the body.
///
/// # Deref
///
/// `Request` implements `Deref<Target = RequestParts>`, so all methods on
/// [`RequestParts`] and [`Parts`] are available directly:
///
/// ```
/// # use beet_core::prelude::*;
/// let request = Request::get("/api/users?limit=10");
/// assert_eq!(request.method(), &HttpMethod::Get);  // From RequestParts
/// assert_eq!(request.path(), &["api", "users"]);   // From Parts via Deref
/// ```
#[derive(Debug, Component)]
#[component(on_add = on_add)]
pub struct Request {
	parts: RequestParts,
	pub body: Body,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let parts = world
		.entity(cx.entity)
		.get::<Request>()
		.unwrap()
		.parts
		.clone();
	world
		.commands()
		.entity(cx.entity)
		.insert(RequestMeta::new(parts));
}

impl std::ops::Deref for Request {
	type Target = RequestParts;
	fn deref(&self) -> &Self::Target { &self.parts }
}

impl std::ops::DerefMut for Request {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parts }
}

/// Cloned from the [`Request`] when its added, allowing the [`Request`]
/// to be consumed and for these parts to still be accessible.
/// This component should not be removed.
#[derive(Debug, Clone, Component)]
pub struct RequestMeta {
	parts: RequestParts,
	/// Note this is taken the moment the request is inserted. It does not account
	/// for the approx 70us overhead created by using bevy at all.
	started: Instant,
}

impl RequestMeta {
	pub fn new(parts: RequestParts) -> Self {
		Self {
			parts,
			started: Instant::now(),
		}
	}

	pub fn method(&self) -> HttpMethod { *self.parts.method() }

	pub fn started(&self) -> Instant { self.started }

	/// Returns a reference to the request parts
	pub fn parts(&self) -> &RequestParts { &self.parts }
}

impl std::ops::Deref for RequestMeta {
	type Target = RequestParts;
	fn deref(&self) -> &Self::Target { &self.parts }
}

impl Request {
	/// Creates a new request with the given method and path
	pub fn new(method: HttpMethod, path: impl AsRef<str>) -> Self {
		Self {
			parts: RequestParts::new(method, path),
			body: default(),
		}
	}

	/// Creates a request from parts and body
	pub fn from_parts(parts: RequestParts, body: Body) -> Self {
		Self { parts, body }
	}

	/// Creates a GET request for the given path
	pub fn get(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Get, path)
	}

	/// Creates a POST request for the given path
	pub fn post(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Post, path)
	}

	/// Creates a PUT request for the given path
	pub fn put(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Put, path)
	}

	/// Creates a DELETE request for the given path
	pub fn delete(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Delete, path)
	}

	/// Creates a PATCH request for the given path
	pub fn patch(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Patch, path)
	}

	/// Creates a HEAD request for the given path
	pub fn head(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Head, path)
	}

	/// Creates an OPTIONS request for the given path
	pub fn options(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Options, path)
	}

	/// Sets the HTTP method
	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.parts = self.parts.with_method(method);
		self
	}

	/// Sets the request body from bytes
	pub fn with_body(mut self, body: impl AsRef<[u8]>) -> Self {
		self.body = Bytes::copy_from_slice(body.as_ref()).into();
		self
	}

	/// Sets the request body from a stream
	pub fn with_body_stream<S>(mut self, stream: S) -> Self
	where
		S: 'static + Send + Sync + futures::Stream<Item = Result<Bytes>>,
	{
		use send_wrapper::SendWrapper;
		self.body = Body::Stream(SendWrapper::new(Box::pin(stream)));
		self
	}

	/// Sets a JSON body and content-type header
	#[cfg(all(feature = "serde", feature = "http"))]
	pub fn with_json_body<T: serde::Serialize>(
		self,
		body: &T,
	) -> Result<Self, serde_json::Error> {
		let body = serde_json::to_string(body)?;
		self.with_body(Bytes::from(body))
			.with_content_type("application/json")
			.xok()
	}

	/// Mutably sets the request body
	pub fn set_body(&mut self, body: impl AsRef<[u8]>) -> &mut Self {
		self.body = Bytes::copy_from_slice(body.as_ref()).into();
		self
	}

	/// Adds a header using http header types
	#[cfg(feature = "http")]
	pub fn with_header<K: IntoHeaderName>(
		mut self,
		key: K,
		value: &str,
	) -> Self {
		let key_str = header_name_to_string(key);
		self.parts.parts_mut().insert_header(key_str, value);
		self
	}

	/// Shorthand for an `Authorization: Bearer <token>` header
	#[cfg(feature = "http")]
	pub fn with_auth_bearer(self, token: &str) -> Self {
		self.with_header(
			http::header::AUTHORIZATION,
			&format!("Bearer {}", token),
		)
	}

	/// Sets the content type header
	#[cfg(feature = "http")]
	pub fn with_content_type(self, content_type: &str) -> Self {
		self.with_header(http::header::CONTENT_TYPE, content_type)
	}

	/// Parse both the key and value as valid URL query parameters
	#[cfg(feature = "serde")]
	pub fn parse_query_param<
		T1: serde::Serialize + ?Sized,
		T2: serde::Serialize,
	>(
		self,
		key: &T1,
		value: &T2,
	) -> Result<Self> {
		let key = serde_urlencoded::to_string(key)?;
		let value = serde_urlencoded::to_string(value)?;
		self.with_query_param(&key, &value).xok()
	}

	/// Insert a query parameter into the request
	pub fn with_query_param(mut self, key: &str, value: &str) -> Self {
		self.parts.parts_mut().insert_param(key, value);
		self
	}

	/// Sets query parameters from a string
	pub fn with_query_string(mut self, query: &str) -> Self {
		for pair in query.split('&') {
			if pair.is_empty() {
				continue;
			}
			let (key, value) = match pair.split_once('=') {
				Some((key, value)) => (key.to_string(), value.to_string()),
				None => (pair.to_string(), String::new()),
			};
			self.parts.parts_mut().insert_param(key, value);
		}
		self
	}

	/// Returns the path as a RoutePath
	pub fn route_path(&self) -> RoutePath {
		RoutePath::new(self.parts.path_string())
	}

	/// Returns a reference to the request parts
	pub fn parts(&self) -> &RequestParts { &self.parts }

	/// Returns a mutable reference to the request parts
	pub fn parts_mut(&mut self) -> &mut RequestParts { &mut self.parts }

	/// Consumes the request and returns the parts and body
	pub fn into_parts(self) -> (RequestParts, Body) { (self.parts, self.body) }

	/// Creates a request from an http::Request
	#[cfg(feature = "http")]
	pub fn from_http<T: Into<Bytes>>(request: http::Request<T>) -> Self {
		let (http_parts, body) = request.into_parts();
		let has_body = http_ext::has_body(&http_parts);
		let parts = RequestParts::from(http_parts);
		let body = if has_body {
			Bytes::from(body.into()).into()
		} else {
			default()
		};
		Self { parts, body }
	}

	/// Creates a request from CLI arguments.
	/// Returns a Result for API compatibility, though parsing always succeeds.
	pub fn from_cli_args(args: CliArgs) -> Result<Self> {
		Ok(Self {
			parts: RequestParts::from(args),
			body: default(),
		})
	}
	pub fn from_cli_str(args: &str) -> Result<Self> {
		let cli_args = CliArgs::parse(args);
		Self::from_cli_args(cli_args)
	}

	/// Converts this request into an http::Request
	#[cfg(feature = "http")]
	pub async fn into_http_request(self) -> Result<http::Request<Bytes>> {
		let bytes = self.body.into_bytes().await?;
		let http_parts: http::request::Parts = self.parts.try_into()?;
		Ok(http::Request::from_parts(http_parts, bytes))
	}
}

/// Helper to convert http header name to string
#[cfg(feature = "http")]
fn header_name_to_string<K: IntoHeaderName>(key: K) -> String {
	// This is a bit of a hack - we create a temporary request to extract the header name
	let mut headers = http::HeaderMap::new();
	headers.insert(key, http::HeaderValue::from_static(""));
	headers
		.keys()
		.next()
		.map(|name| name.to_string())
		.unwrap_or_default()
}

impl From<&str> for Request {
	fn from(path: &str) -> Self { Request::get(path) }
}

impl From<CliArgs> for Request {
	fn from(args: CliArgs) -> Self {
		Self {
			parts: RequestParts::from(args),
			body: default(),
		}
	}
}

/// Types which consume a request, requiring its body which may be a stream
pub trait FromRequest<M>: Sized {
	fn from_request(
		request: Request,
	) -> MaybeSendBoxedFuture<'static, Result<Self, Response>>;
}

pub struct TryFromRequestMarker;

impl<T, E, M> FromRequest<(E, M, TryFromRequestMarker)> for T
where
	T: TryFrom<Request, Error = E>,
	E: IntoResponse<M>,
{
	fn from_request(
		request: Request,
	) -> MaybeSendBoxedFuture<'static, Result<Self, Response>> {
		Box::pin(async move {
			request.try_into().map_err(|err: E| err.into_response())
		})
	}
}

/// Types which consume a request by reference, not requiring its body
pub trait FromRequestMeta<M>: Sized {
	fn from_request_meta(request: &RequestMeta) -> Result<Self, Response>;
}

impl FromRequestMeta<Self> for () {
	fn from_request_meta(_request: &RequestMeta) -> Result<Self, Response> {
		Ok(())
	}
}

pub struct FromRequestMetaMarker;

impl<T, M> FromRequest<(FromRequestMetaMarker, M)> for T
where
	T: FromRequestMeta<M>,
{
	fn from_request(
		request: Request,
	) -> MaybeSendBoxedFuture<'static, Result<Self, Response>> {
		let meta = RequestMeta::new(request.parts);
		Box::pin(async move { T::from_request_meta(&meta) })
	}
}

#[cfg(feature = "http")]
impl<T: Into<Bytes>> From<http::Request<T>> for Request {
	fn from(request: http::Request<T>) -> Self { Self::from_http(request) }
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn request_get() {
		let request = Request::get("/api/users");
		(*request.method()).xpect_eq(HttpMethod::Get);
		request
			.path()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
	}

	#[test]
	fn request_post() {
		let request = Request::post("/api/users");
		(*request.method()).xpect_eq(HttpMethod::Post);
	}

	#[test]
	fn request_with_method() {
		let request =
			Request::get("/api/users").with_method(HttpMethod::Delete);
		(*request.method()).xpect_eq(HttpMethod::Delete);
	}

	#[test]
	fn request_with_body() {
		let request = Request::post("/api/users").with_body(b"hello");
		request
			.body
			.bytes_eq(&Body::Bytes(Bytes::from("hello")))
			.xpect_true();
	}

	#[test]
	fn request_deref_to_parts() {
		let request = Request::get("/api/users");
		// Should be able to call Parts methods via Deref
		request.path().len().xpect_eq(2);
		request.path_string().xpect_eq("/api/users");
		request.scheme().clone().xpect_eq(Scheme::None);
	}

	#[test]
	fn request_from_cli_args() {
		let cli = CliArgs::parse("users list --limit 10");
		let request = Request::from(cli);

		(*request.method()).xpect_eq(HttpMethod::Get);
		request.scheme().clone().xpect_eq(Scheme::Cli);
		request
			.path()
			.xpect_eq(vec!["users".to_string(), "list".to_string()]);
		request.get_param("limit").unwrap().xpect_eq("10");
	}

	#[test]
	#[cfg(feature = "http")]
	fn request_from_http() {
		let http_request = http::Request::builder()
			.method(http::Method::POST)
			.uri("/api/users?page=1")
			.header("content-type", "application/json")
			.body(Bytes::from("{}"))
			.unwrap();

		let request = Request::from_http(http_request);

		(*request.method()).xpect_eq(HttpMethod::Post);
		request
			.path()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		request.get_param("page").unwrap().xpect_eq("1");
	}

	#[test]
	fn request_with_query_param() {
		let request = Request::get("/api/users")
			.with_query_param("limit", "10")
			.with_query_param("offset", "20");

		request.get_param("limit").unwrap().xpect_eq("10");
		request.get_param("offset").unwrap().xpect_eq("20");
	}

	#[test]
	fn request_route_path() {
		let request = Request::get("/api/users/123");
		request.route_path().to_string().xpect_eq("/api/users/123");
	}

	#[test]
	fn request_from_str() {
		let request: Request = "/api/users".into();
		(*request.method()).xpect_eq(HttpMethod::Get);
		request.path_string().xpect_eq("/api/users");
	}

	#[test]
	fn request_into_parts() {
		let request = Request::post("/api/users").with_body(b"data");
		let (parts, body) = request.into_parts();

		(*parts.method()).xpect_eq(HttpMethod::Post);
		body.bytes_eq(&Body::Bytes(Bytes::from("data")))
			.xpect_true();
	}
}

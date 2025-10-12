use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use http::Uri;
use http::header::IntoHeaderName;
use http::request;
use std::str::FromStr;

/// A generalized request [`Resource`] added to every route app before the
/// request is processed.
#[derive(Debug, Component, Resource)]
#[component(on_add=on_add)]
pub struct Request {
	pub parts: request::Parts,
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

/// Cloned from the [`Request`] when its added, allowing the [`Request`]
/// to be consumed and for these parts to still be accessible.
/// This component should not be removed
#[derive(Debug, Component)]
pub struct RequestMeta {
	parts: request::Parts,
	/// Note this is taken the moment the request is inserted. It does not account
	/// for the approx 70us overhead created by using bevy at all.
	started: Instant,
}
impl RequestMeta {
	pub fn new(parts: request::Parts) -> Self {
		Self {
			parts,
			started: Instant::now(),
		}
	}
	pub fn method(&self) -> HttpMethod { self.parts.method.clone().into() }
	pub fn path(&self) -> &str { self.parts.uri.path() }
	pub fn started(&self) -> Instant { self.started }
	pub fn parts(&self) -> &request::Parts { &self.parts }
}


impl Request {
	pub fn new(method: HttpMethod, path: impl AsRef<str>) -> Self {
		let parts = request::Builder::new()
			.method(method)
			.uri(path.as_ref())
			.body(())
			.expect("Failed to create request parts")
			.into_parts()
			.0;
		Self {
			parts,
			body: default(),
		}
	}


	pub fn from_parts(parts: request::Parts, body: Body) -> Self {
		Self { parts, body }
	}

	pub fn get(path: impl AsRef<str>) -> Self {
		Self {
			parts: request::Builder::new()
				.method(http::Method::GET)
				.uri(path.as_ref())
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: default(),
		}
	}
	pub fn post(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Post)
	}
	pub fn put(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Put)
	}
	pub fn delete(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Delete)
	}
	pub fn patch(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Patch)
	}
	pub fn head(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Head)
	}
	pub fn options(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Options)
	}

	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.parts.method = method.into();
		self
	}

	pub fn with_body(mut self, body: impl AsRef<[u8]>) -> Self {
		self.body = Bytes::copy_from_slice(body.as_ref()).into();
		self
	}

	pub fn with_body_stream<S>(mut self, stream: S) -> Self
	where
		S: 'static + Send + Sync + futures::Stream<Item = Result<Bytes>>,
	{
		use send_wrapper::SendWrapper;
		self.body = Body::Stream(SendWrapper::new(Box::pin(stream)));
		self
	}

	/// Add a json body, and set the content type to application/json
	#[cfg(feature = "serde")]
	pub fn with_json_body<T: serde::Serialize>(
		self,
		body: &T,
	) -> Result<Self, serde_json::Error> {
		use beet_core::prelude::*;

		let body = serde_json::to_string(body)?;
		self.with_body(body)
			.with_content_type("application/json")
			.xok()
	}

	pub fn set_body(&mut self, body: impl AsRef<[u8]>) -> &mut Self {
		self.body = Bytes::copy_from_slice(body.as_ref()).into();
		self
	}
	pub fn with_header<K: IntoHeaderName>(
		mut self,
		key: K,
		value: &str,
	) -> Self {
		self.parts
			.headers
			.insert(key, http::header::HeaderValue::from_str(value).unwrap());
		self
	}

	/// Shorthand for an `Authorization: Bearer <token>` header.
	pub fn with_auth_bearer(mut self, token: &str) -> Self {
		self.parts.headers.insert(
			http::header::AUTHORIZATION,
			http::header::HeaderValue::from_str(&format!("Bearer {}", token))
				.unwrap(),
		);
		self
	}

	/// Set the content type header.
	pub fn with_content_type(mut self, content_type: &str) -> Self {
		self.parts.headers.insert(
			http::header::CONTENT_TYPE,
			http::header::HeaderValue::from_str(content_type).unwrap(),
		);
		self
	}

	/// Parse both the key and value as valid URL query parameters.
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
		self.with_query_param(&key, &value)
	}

	/// Insert a query parameter into the request URI without checking it is
	/// a valid URL, for the checked version use [`Self::parse_query_param`].
	pub fn with_query_param(self, key: &str, value: &str) -> Result<Self> {
		let updated_query = if let Some(query) = self.parts.uri.query() {
			format!("{}&{}={}", query, key, value)
		} else {
			format!("{}={}", key, value)
		};
		self.with_query_string(&updated_query)
	}
	pub fn with_query_string(mut self, query: &str) -> Result<Self> {
		let path = self.parts.uri.path();
		let path_and_query = format!("{}?{}", path, query);
		let mut uri_parts = self.parts.uri.clone().into_parts();
		uri_parts.path_and_query =
			Some(http::uri::PathAndQuery::from_str(&path_and_query)?);
		self.parts.uri = Uri::from_parts(uri_parts)?;
		Ok(self)
	}

	pub fn method(&self) -> HttpMethod {
		HttpMethod::from(self.parts.method.clone())
	}

	pub fn from_http<T: Into<Bytes>>(request: http::Request<T>) -> Self {
		let (parts, body) = request.into_parts();
		let body = if HttpExt::has_body(&parts) {
			Bytes::from(body.into()).into()
		} else {
			default()
		};
		Self { parts, body }
	}

	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub async fn from_axum<S: 'static + Send + Sync>(
		request: axum::extract::Request,
		state: &S,
	) -> HttpResult<Self> {
		use axum::extract::FromRequest;
		let (parts, body) = request.into_parts();
		let body = if HttpExt::has_body(&parts) {
			let request =
				axum::extract::Request::from_parts(parts.clone(), body);
			let bytes =
				Bytes::from_request(request, state).await.map_err(|err| {
					HttpError::bad_request(format!(
						"Failed to extract request: {}",
						err
					))
				})?;
			bytes.into()
		} else {
			default()
		};
		Ok(Self { parts, body })
	}
	pub async fn into_http_request(self) -> Result<http::Request<Bytes>> {
		let bytes = self.body.into_bytes().await?;
		Ok(http::Request::from_parts(self.parts, bytes))
	}
}


impl From<RouteInfo> for Request {
	fn from(route_info: RouteInfo) -> Self {
		let method: http::Method = route_info.method.into();
		Self {
			parts: request::Builder::new()
				.method(method)
				.uri(route_info.path.to_string_lossy().as_ref())
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: default(),
		}
	}
}

impl Into<()> for Request {
	fn into(self) -> () {}
}


impl From<&str> for Request {
	fn from(path: &str) -> Self { Request::get(path) }
}

/// Types which consume a request, requiring its body which may be a stream
pub trait FromRequest<M>: Sized {
	fn from_request(
		request: Request,
	) -> SendBoxedFuture<Result<Self, Response>>;
	// temp while migrating beet_router
	fn from_request_sync(request: Request) -> Result<Self, Response> {
		futures::executor::block_on(Self::from_request(request))
	}
}

/// Types which consume a request by reference, not requiring its body
pub trait FromRequestRef<M>: Sized {
	fn from_request_ref(request: &Request) -> Result<Self, Response>;
}
pub struct TryFromRequestMarker;

impl<T, E, M> FromRequest<(E, M, TryFromRequestMarker)> for T
where
	T: TryFrom<Request, Error = E>,
	E: IntoResponse<M>,
{
	fn from_request(
		request: Request,
	) -> SendBoxedFuture<Result<Self, Response>> {
		Box::pin(
			async move { request.try_into().map_err(|e: E| e.into_response()) },
		)
	}
}

pub struct FromRequestRefMarker;

impl<T, M> FromRequest<(FromRequestRefMarker, M)> for T
where
	T: FromRequestRef<M>,
{
	fn from_request(
		request: Request,
	) -> SendBoxedFuture<Result<Self, Response>> {
		Box::pin(async move { T::from_request_ref(&request) })
	}
}

impl<T, E, M> FromRequestRef<(E, M)> for T
where
	T: for<'a> TryFrom<&'a Request, Error = E>,
	E: IntoResponse<M>,
{
	fn from_request_ref(request: &Request) -> Result<Self, Response> {
		request.try_into().map_err(|e: E| e.into_response())
	}
}

impl<T: Into<Bytes>> From<http::Request<T>> for Request {
	fn from(request: http::Request<T>) -> Self { Self::from_http(request) }
}

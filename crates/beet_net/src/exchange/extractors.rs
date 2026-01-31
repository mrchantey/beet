//! Extractors for declaratively converting parts of an exchange into concrete types.
//!
//! Extractors simplify request handling by automatically parsing and validating
//! parts of the request (query params, body, headers) into typed Rust values.
//!
//! ## Available Extractors
//!
//! - [`QueryParams`]: Parse URL query parameters into a struct
//! - [`JsonQueryParams`]: Parse complex types encoded as JSON in query params
//! - [`Json`]: Parse JSON request body
//! - [`Html`], [`Css`], [`Javascript`], [`Png`]: Response type wrappers
#[allow(unused)]
use beet_core::prelude::*;

/// Wrapper for HTML content responses.
pub struct Html<T>(pub T);

/// Wrapper for CSS content responses.
pub struct Css(pub String);

/// Wrapper for JavaScript content responses.
pub struct Javascript(pub String);

/// Wrapper for PNG image responses.
pub struct Png(pub String);


impl Into<Html<Self>> for String {
	fn into(self) -> Html<Self> { Html(self) }
}
impl Into<Css> for String {
	fn into(self) -> Css { Css(self) }
}
impl Into<Javascript> for String {
	fn into(self) -> Javascript { Javascript(self) }
}
impl Into<Png> for String {
	fn into(self) -> Png { Png(self) }
}

/// A result type that includes an HTTP status code for error responses.
///
/// When a server action fails and the error should be returned to the client,
/// this type allows specifying both the error value and the HTTP status code.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JsonResult<T, E> {
	/// The inner result value.
	pub result: Result<T, E>,
	/// The status code to return in case of an error.
	/// Defaults to 418 (I'm a teapot).
	#[cfg_attr(feature = "serde", serde(with = "status_code_serde"))]
	pub err_status: StatusCode,
}


impl JsonResult<(), ()> {
	/// Default error status code when the `http` feature is enabled.
	#[cfg(feature = "http")]
	pub const DEFAULT_ERR_STATUS: StatusCode = StatusCode::ImATeapot;
	/// Default error status code when the `http` feature is disabled.
	#[cfg(not(feature = "http"))]
	pub const DEFAULT_ERR_STATUS: StatusCode = StatusCode::InternalError;
}

impl<T, E> From<Result<T, E>> for JsonResult<T, E> {
	fn from(result: Result<T, E>) -> Self {
		Self {
			result,
			err_status: JsonResult::DEFAULT_ERR_STATUS,
		}
	}
}

impl<T, E> JsonResult<T, E> {
	/// Creates a new [`JsonResult`] from a result.
	pub fn new(val: Result<T, E>) -> Self { Self::from(val) }

	/// Convenience function for system piping.
	pub fn pipe(val: In<Result<T, E>>) -> Self { Self::from(val.0) }

	/// Creates a pipe function with a custom error status code.
	pub fn pipe_with_status(
		status: StatusCode,
	) -> impl Fn(In<Result<T, E>>) -> Self {
		move |val: In<Result<T, E>>| Self {
			result: val.0,
			err_status: status,
		}
	}
}
#[cfg(feature = "serde")]
impl<T: serde::Serialize, E: serde::Serialize> TryInto<Response>
	for JsonResult<T, E>
{
	type Error = HttpError;

	fn try_into(self) -> Result<Response, Self::Error> {
		match self.result {
			Ok(val) => {
				let ok_body = serde_json::to_string(&val)?;
				Response::ok_body(ok_body, "application/json")
			}
			Err(err) => {
				let err_body = serde_json::to_string(&err)?;
				Response::from_status_body(
					self.err_status,
					&err_body,
					"application/json",
				)
			}
		}
		.xok()
	}
}

/// Wrapper for JSON request/response bodies.
///
/// Use this to automatically serialize/deserialize JSON in handlers.
#[derive(Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Json<T>(pub T);

impl<T> Json<T> {
	/// Creates a new JSON wrapper.
	pub fn new(val: T) -> Self { Self(val) }

	/// Convenience function for system piping.
	pub fn pipe(val: In<T>) -> Json<T> { Json(val.0) }
}


#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> FromRequest<Self> for Json<T> {
	fn from_request(
		req: Request,
	) -> MaybeSendBoxedFuture<'static, Result<Self, Response>> {
		Box::pin(async move {
			let body = req.body.into_bytes().await.map_err(|err| {
				error!("Failed to read request body: {}", err);
				HttpError::bad_request("Failed to read stream")
			})?;
			let json = serde_json::from_slice(&body).map_err(|err| {
				HttpError::bad_request(format!("Failed to parse JSON: {}", err))
			})?;
			Ok(Self(json))
		})
	}
}
#[cfg(feature = "serde")]
impl<T: serde::Serialize> TryInto<Response> for Json<T> {
	type Error = HttpError;

	fn try_into(self) -> Result<Response, Self::Error> {
		let json_str = serde_json::to_string(&self.0)?;
		Ok(Response::ok_body(
			json_str,
			"application/json; charset=utf-8",
		))
	}
}


/// Query params wrapper that supports complex types via JSON encoding.
///
/// [`QueryParams`] is limited (no enums or tuples). This type accepts any
/// serializable value by first converting to JSON, then URL-encoding.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JsonQueryParams<T>(pub T);

/// Internal representation for JSON query params.
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct JsonQueryParamsInner {
	data: String,
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> JsonQueryParams<T> {
	/// Serializes a value to a URL-encoded query string.
	pub fn to_query_string(value: &T) -> Result<String> {
		let data = serde_json::to_string(value)?;
		serde_urlencoded::to_string(&JsonQueryParamsInner { data })?.xok()
	}
}

#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> JsonQueryParams<T> {
	/// Deserializes a value from a URL-encoded query string.
	pub fn from_query_string(query: &str) -> Result<T> {
		let inner = serde_urlencoded::from_str::<JsonQueryParamsInner>(query)?;
		serde_json::from_str::<T>(&inner.data)?.xok()
	}
}
#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> FromRequestMeta<Self>
	for JsonQueryParams<T>
{
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		let query = req.query_string();
		if query.is_empty() {
			return Err(
				HttpError::bad_request("no query params in request").into()
			);
		}
		let value = Self::from_query_string(&query).map_err(|err| {
			HttpError::bad_request(format!(
				"Failed to parse query params: {}",
				err
			))
		})?;
		Ok(Self(value))
	}
}
/// An extractor to represent query params as a struct.
/// # Example
/// ```
/// # use beet_net::prelude::*;
/// # use serde::Deserialize;
/// #[derive(Deserialize)]
/// struct MyParams {
/// 	name: String,
/// }
///
/// fn my_route(params: QueryParams<MyParams>) -> String {
///   params.name.clone()
/// }
/// ```
#[derive(Debug, Clone, Deref)]
pub struct QueryParams<T>(pub T);

#[cfg(feature = "serde")]
impl<T: serde::Serialize> QueryParams<T> {
	/// Encodes the params as a URL-encoded query string.
	pub fn encode(&self) -> Result<String> {
		serde_urlencoded::to_string(&self.0)?.xok()
	}
}

#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> QueryParams<T> {
	/// Decodes a URL-encoded query string into the specified type.
	pub fn decode(value: &str) -> Result<T> {
		serde_urlencoded::from_str::<T>(value)?.xok()
	}
}

#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> FromRequestMeta<Self> for QueryParams<T> {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		let query = req.query_string();
		if query.is_empty() {
			return Err(
				HttpError::bad_request("no query params in request").into()
			);
		}
		let params: T = serde_urlencoded::from_str(&query).map_err(|err| {
			HttpError::bad_request(format!(
				"Failed to parse query params: {}",
				err
			))
		})?;
		Ok(Self(params))
	}
}



impl<T> Into<Response> for Html<T>
where
	T: Into<Body>,
{
	fn into(self) -> Response {
		Response::ok_body(self.0, "text/html; charset=utf-8")
	}
}

impl Into<Response> for Css {
	fn into(self) -> Response {
		Response::ok_body(self.0, "text/css; charset=utf-8")
	}
}

impl Into<Response> for Javascript {
	fn into(self) -> Response {
		Response::ok_body(self.0, "application/javascript; charset=utf-8")
	}
}

impl Into<Response> for Png {
	fn into(self) -> Response { Response::ok_body(self.0, "image/png") }
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn request_response_cycle() {
		let mut app = App::new();
		let req = Request::post("/test")
			.with_header("content-length", "5")
			.with_body(b"hello");

		let entity = app.world_mut().spawn(req).id();
		app.add_systems(
			Update,
			move |mut commands: Commands, query: Query<&Request>| {
				let _req = query.single().unwrap();
				let res = Response::ok().with_header("content-length", "5");
				commands.entity(entity).insert(res);
			},
		);
		app.update();

		app.world_mut()
			.entity_mut(entity)
			.take::<Response>()
			.unwrap()
			.get_header("content-length")
			.unwrap()
			.xpect_eq("5");
	}

	#[test]
	fn parts_has_body() {
		let parts = PartsBuilder::new()
			.path_str("/test")
			.header("content-length", "5")
			.build_request_parts(HttpMethod::Post);

		parts.has_body().xpect_true();

		let parts_without = RequestParts::get("/test");
		parts_without.has_body().xpect_false();
	}

	#[test]
	#[cfg(feature = "serde")]
	fn json_query_params() {
		#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
		struct Foo(u32, String);
		let val = Foo(42, "foo$\" \" &dsds?sd#@$)#@$*()".to_owned());

		let query_str = JsonQueryParams::to_query_string(&val).unwrap();
		(&query_str).xpect_starts_with("data=%5B42%2C%22foo");
		for str in &[" ", "$", "\"", "&", "?", "#", "@", "(", ")"] {
			(&query_str).xnot().xpect_contains(str);
		}

		let val2 =
			JsonQueryParams::<Foo>::from_query_string(&query_str).unwrap();
		val.xpect_eq(val2);
	}
}

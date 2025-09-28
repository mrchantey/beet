use crate::prelude::*;
#[allow(unused)]
use beet_core::prelude::*;
use http::HeaderMap;
use http::StatusCode;
use http::request;

pub struct Html(pub String);
pub struct Css(pub String);
pub struct Javascript(pub String);
pub struct Png(pub String);


impl Into<Html> for String {
	fn into(self) -> Html { Html(self) }
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

/// When a server action fails and the error should be returned, its also good
/// practice to return a status code indicating the issue.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JsonResult<T, E> {
	pub result: Result<T, E>,
	/// The status code to return in case of an error,
	/// defaults to 418 (I'm a teapot).
	#[cfg_attr(feature = "serde", serde(with = "status_code_serde"))]
	pub err_status: StatusCode,
}


impl JsonResult<(), ()> {
	pub const DEFAULT_ERR_STATUS: StatusCode = StatusCode::IM_A_TEAPOT;
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
	pub fn new(val: Result<T, E>) -> Self { Self::from(val) }
	/// Convenience function for system piping
	pub fn pipe(val: In<Result<T, E>>) -> Self { Self::from(val.0) }
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
				Response::ok_body(&ok_body, "application/json")
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

#[derive(Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Json<T>(pub T);

impl<T> Json<T> {
	pub fn new(val: T) -> Self { Self(val) }
	/// Convenience function for system piping
	pub fn pipe(val: In<T>) -> Json<T> { Json(val.0) }
}

#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> std::convert::TryFrom<Request>
	for Json<T>
{
	type Error = HttpError;

	fn try_from(req: Request) -> std::result::Result<Self, Self::Error> {
		let body = req
			.body
			.ok_or_else(|| HttpError::bad_request("no body in request"))?;
		let json: T = serde_json::from_slice(&body).map_err(|err| {
			HttpError::bad_request(format!("Failed to parse JSON: {}", err))
		})?;
		Ok(Json(json))
	}
}
#[cfg(feature = "serde")]
impl<T: serde::Serialize> TryInto<Response> for Json<T> {
	type Error = HttpError;

	fn try_into(self) -> Result<Response, Self::Error> {
		let json_str = serde_json::to_string(&self.0)?;
		Ok(Response::ok_body(
			&json_str,
			"application/json; charset=utf-8",
		))
	}
}


/// [`QueryParams`] is a limited format, for example enums and tuples are not allowed,
/// this struct accepts any value by first serializing it as JSON,
/// then encode it as a URL-encoded string, for use as a query param value.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JsonQueryParams<T>(pub T);

/// The query params representation of the [`JsonQueryParams`].
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct JsonQueryParamsInner {
	data: String,
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> JsonQueryParams<T> {
	pub fn to_query_string(value: &T) -> Result<String> {
		let data = serde_json::to_string(value)?;
		serde_urlencoded::to_string(&JsonQueryParamsInner { data })?.xok()
	}
}

#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> JsonQueryParams<T> {
	pub fn from_query_string(query: &str) -> Result<T> {
		let inner = serde_urlencoded::from_str::<JsonQueryParamsInner>(query)?;
		serde_json::from_str::<T>(&inner.data)?.xok()
	}
}
#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> std::convert::TryFrom<Request>
	for JsonQueryParams<T>
{
	type Error = HttpError;

	fn try_from(req: Request) -> std::result::Result<Self, Self::Error> {
		let query = req.parts.uri.query().ok_or_else(|| {
			HttpError::bad_request("no query params in request")
		})?;
		let value = Self::from_query_string(query)?;
		Ok(Self(value))
	}
}

pub struct QueryParams<T>(pub T);

#[cfg(feature = "serde")]
impl<T: serde::Serialize> QueryParams<T> {
	/// Parses as serde_json and encodes the data as a URL-encoded string,
	/// for use as a query param value.
	pub fn encode(&self) -> Result<String> {
		serde_urlencoded::to_string(&self.0)?.xok()
	}
}
#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> QueryParams<T> {
	/// Decodes a URL-encoded string into a serde_json value,
	/// then deserializes it into the specified type.
	pub fn decode(value: &str) -> Result<T> {
		serde_urlencoded::from_str::<T>(value)?.xok()
	}
}

#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> std::convert::TryFrom<Request>
	for QueryParams<T>
{
	type Error = HttpError;

	fn try_from(req: Request) -> std::result::Result<Self, Self::Error> {
		let query = req.parts.uri.query().ok_or_else(|| {
			HttpError::bad_request("no query params in request")
		})?;
		let params: T = serde_urlencoded::from_str(query).map_err(|err| {
			HttpError::bad_request(format!(
				"Failed to parse query params: {}",
				err
			))
		})?;
		Ok(Self(params))
	}
}


impl<'a> Into<Response> for &'a str {
	fn into(self) -> Response {
		Response::ok_body(self, "text/plain; charset=utf-8")
	}
}

impl Into<Response> for String {
	fn into(self) -> Response {
		Response::ok_body(&self, "text/plain; charset=utf-8")
	}
}

impl Into<Response> for Html {
	fn into(self) -> Response {
		Response::ok_body(&self.0, "text/html; charset=utf-8")
	}
}

impl Into<Response> for Css {
	fn into(self) -> Response {
		Response::ok_body(&self.0, "text/css; charset=utf-8")
	}
}

impl Into<Response> for Javascript {
	fn into(self) -> Response {
		Response::ok_body(&self.0, "application/javascript; charset=utf-8")
	}
}

impl Into<Response> for Png {
	fn into(self) -> Response { Response::ok_body(&self.0, "image/png") }
}


// this would be DeriveAppState
pub struct RouteApp {
	// pub create_app: Box<dyn Clone + Fn() -> App>,
}

pub struct HttpExt;

impl HttpExt {
	pub fn has_body(parts: &request::Parts) -> bool {
		Self::has_body_by_content_length(&parts.headers)
			|| Self::has_body_by_transfer_encoding(&parts.headers)
	}

	pub fn has_body_by_content_length(headers: &HeaderMap) -> bool {
		headers
			.get("content-length")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<usize>().ok())
			.map(|len| len > 0)
			.unwrap_or(false)
	}

	pub fn has_body_by_transfer_encoding(headers: &HeaderMap) -> bool {
		headers
			.get("transfer-encoding")
			.and_then(|v| v.to_str().ok())
			.map(|s| s.contains("chunked"))
			.unwrap_or(false)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use bytes::Bytes;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		let req: Request = http::Request::builder()
			.method(http::Method::POST)
			.uri("https://example.com")
			.header("content-length", "5")
			.body(Bytes::new())
			.unwrap()
			.into();
		app.insert_resource(req);
		app.add_systems(Update, |mut commands: Commands, req: Res<Request>| {
			let mut res = Response::ok();
			res.parts.headers = req.parts.headers.clone();
			commands.insert_resource(res);
		});
		app.update();

		let res = app.world_mut().remove_resource::<Response>().unwrap();
		res.parts
			.headers
			.get("content-length")
			.unwrap()
			.xpect_eq("5");
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

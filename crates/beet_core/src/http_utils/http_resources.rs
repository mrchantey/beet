use bevy::prelude::*;
use bytes::Bytes;
use http::HeaderMap;
use http::StatusCode;
use http::request;
use http::response;

use crate::prelude::RouteInfo;

/// A generalized request [`Resource`] added to every route app before the
/// request is processed.
#[derive(Debug, Clone, Resource)]
pub struct Request {
	pub parts: request::Parts,
	pub body: Option<Bytes>,
}

impl Request {
	pub fn new(parts: request::Parts, body: Option<Bytes>) -> Self {
		Self { parts, body }
	}

	pub fn from_http<T: Into<Bytes>>(request: http::Request<T>) -> Self {
		let (parts, body) = request.into_parts();
		let bytes = if HttpExt::has_body(&parts) {
			Some(Bytes::from(body.into()))
		} else {
			None
		};
		Self { parts, body: bytes }
	}

	#[cfg(feature = "server")]
	pub async fn from_axum<S: 'static + Send + Sync>(
		request: axum::extract::Request,
		state: &S,
	) -> Result<Self, axum::extract::rejection::BytesRejection> {
		use axum::extract::FromRequest;
		let (parts, body) = request.into_parts();
		let bytes = if HttpExt::has_body(&parts) {
			let request =
				axum::extract::Request::from_parts(parts.clone(), body);
			let bytes = Bytes::from_request(request, state).await?;
			Some(bytes)
		} else {
			None
		};
		Ok(Self { parts, body: bytes })
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
			body: None,
		}
	}
}

impl<T: Into<Bytes>> From<http::Request<T>> for Request {
	fn from(request: http::Request<T>) -> Self { Self::from_http(request) }
}

/// Added by the route or its layers, otherwise an empty [`StatusCode::Ok`]
/// will be returned.
#[derive(Debug, Clone, Resource)]
pub struct Response {
	pub parts: response::Parts,
	pub body: Option<Bytes>,
}




pub struct Html(pub String);
pub struct Css(pub String);
pub struct Javascript(pub String);
pub struct Json(pub String);
pub struct Png(pub String);

/// Allows for blanket implementation of `Into<Response>` for various types
/// and their `Result` variants.
pub trait IntoResponse {
	fn into_response(self) -> Response;
}

impl<T: Into<Response>> IntoResponse for T {
	fn into_response(self) -> Response { self.into() }
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
	fn into_response(self) -> Response {
		match self {
			Ok(t) => t.into_response(),
			Err(e) => e.into_response(),
		}
	}
}

impl IntoResponse for BevyError {
	fn into_response(self) -> Response {
		// Log the error and do not return to the client
		error!("BevyError: {}", self.to_string());
		Response::from_parts(
			http::response::Builder::new()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			// do not assume a bevy error message is safe to return to the client
			Some(Bytes::from("Internal Bevy Error")),
		)
	}
}

impl<'a> IntoResponse for &'a str {
	fn into_response(self) -> Response {
		Response::ok_str(self, "text/plain; charset=utf-8")
	}
}

impl IntoResponse for String {
	fn into_response(self) -> Response {
		Response::ok_str(&self, "text/plain; charset=utf-8")
	}
}

impl IntoResponse for Html {
	fn into_response(self) -> Response {
		Response::ok_str(&self.0, "text/html; charset=utf-8")
	}
}

impl IntoResponse for Css {
	fn into_response(self) -> Response {
		Response::ok_str(&self.0, "text/css; charset=utf-8")
	}
}

impl IntoResponse for Javascript {
	fn into_response(self) -> Response {
		Response::ok_str(&self.0, "application/javascript; charset=utf-8")
	}
}

impl IntoResponse for Json {
	fn into_response(self) -> Response {
		Response::ok_str(&self.0, "application/json; charset=utf-8")
	}
}

impl IntoResponse for Png {
	fn into_response(self) -> Response {
		Response::ok_str(&self.0, "image/png")
	}
}

impl Response {
	pub fn from_status_body(status: StatusCode, body: &[u8]) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(status)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			Some(Bytes::copy_from_slice(body)),
		)
	}


	pub fn from_parts(parts: response::Parts, body: Option<Bytes>) -> Self {
		Self { parts, body }
	}

	/// Create a response returning a string body with a 200 OK status.
	pub fn ok_str(body: &str, content_type: &str) -> Self {
		Self {
			parts: http::response::Builder::new()
				.status(StatusCode::OK)
				.header(http::header::CONTENT_TYPE, content_type)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: Some(Bytes::copy_from_slice(body.as_bytes())),
		}
	}

	pub fn body_str(self) -> Option<String> {
		self.body
			.map(|b| String::from_utf8(b.to_vec()).unwrap_or_default())
	}

	pub fn into_http(self) -> http::Response<Bytes> {
		http::Response::from_parts(
			self.parts,
			self.body.unwrap_or_else(|| Bytes::new()),
		)
	}

	#[cfg(feature = "server")]
	pub fn into_axum(self) -> axum::response::Response {
		axum::response::Response::from_parts(
			self.parts,
			self.body.map_or_else(
				|| axum::body::Body::empty(),
				|bytes| axum::body::Body::from(bytes),
			),
		)
	}
}

impl Into<http::Response<Bytes>> for Response {
	fn into(self) -> http::Response<Bytes> { self.into_http() }
}

#[cfg(feature = "server")]
impl Into<axum::response::Response> for Response {
	fn into(self) -> axum::response::Response { self.into_axum() }
}

impl Default for Response {
	fn default() -> Self {
		Self {
			// one does not simply Parts::default()
			parts: http::response::Builder::new()
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: None,
		}
	}
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
	use crate::http_resources::Request;
	use crate::http_resources::Response;
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
			let mut res = Response::default();
			res.parts.headers = req.parts.headers.clone();
			commands.insert_resource(res);
		});
		app.update();

		let res = app.world_mut().remove_resource::<Response>().unwrap();
		res.parts
			.headers
			.get("content-length")
			.unwrap()
			.xpect()
			.to_be("5");
	}
}

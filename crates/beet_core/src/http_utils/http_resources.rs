use bevy::prelude::*;
use bytes::Bytes;
use http::HeaderMap;
use http::StatusCode;
use http::request;
use http::response;

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

impl Response {
	pub fn new(parts: response::Parts, body: Option<Bytes>) -> Self {
		Self { parts, body }
	}

	/// Create a response returning a string body with a 200 OK status.
	pub fn new_str(body: &str) -> Self {
		Self {
			parts: http::response::Builder::new()
				.status(StatusCode::OK)
				.header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: Some(Bytes::copy_from_slice(body.as_bytes())),
		}
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

use crate::prelude::*;
use bevy::prelude::*;
use http::HeaderMap;
use http::request;

pub struct Html(pub String);
pub struct Css(pub String);
pub struct Javascript(pub String);
pub struct Json<T>(pub T);
pub struct QueryParams<T>(pub T);
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



#[cfg(feature = "serde")]
impl<T: serde::de::DeserializeOwned> std::convert::TryFrom<Request> for Json<T> {
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
impl<T: serde::de::DeserializeOwned> std::convert::TryFrom<Request> for QueryParams<T> {
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
		Ok(QueryParams(params))
	}
}


#[cfg(feature = "serde")]
impl<T: serde::Serialize> TryInto<Response> for Json<T> {
	type Error = HttpError;

	fn try_into(self) -> Result<Response, Self::Error> {
		let json_str = serde_json::to_string(&self.0)?;
		Ok(Response::ok_str(
			&json_str,
			"application/json; charset=utf-8",
		))
	}
}

impl<'a> Into<Response> for &'a str {
	fn into(self) -> Response {
		Response::ok_str(self, "text/plain; charset=utf-8")
	}
}

impl Into<Response> for String {
	fn into(self) -> Response {
		Response::ok_str(&self, "text/plain; charset=utf-8")
	}
}

impl Into<Response> for Html {
	fn into(self) -> Response {
		Response::ok_str(&self.0, "text/html; charset=utf-8")
	}
}

impl Into<Response> for Css {
	fn into(self) -> Response {
		Response::ok_str(&self.0, "text/css; charset=utf-8")
	}
}

impl Into<Response> for Javascript {
	fn into(self) -> Response {
		Response::ok_str(&self.0, "application/javascript; charset=utf-8")
	}
}

impl Into<Response> for Png {
	fn into(self) -> Response { Response::ok_str(&self.0, "image/png") }
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
			.xpect()
			.to_be("5");
	}
}

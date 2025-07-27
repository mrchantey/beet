use crate::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use http::HeaderValue;
use http::StatusCode;
use http::header::HeaderName;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;


fn js_err(err: JsValue) -> BevyError { bevyhow!("{err:?}") }

pub(crate) async fn send_wasm(request: Request) -> Result<Response> {
	let request: web_sys::Request = request.try_into()?;
	let window = web_sys::window().ok_or_else(|| bevyhow!("No window"))?;
	let promise = window.fetch_with_request(&request);
	let resp_js = wasm_bindgen_futures::JsFuture::from(promise)
		.await
		.map_err(js_err)?;
	let resp = resp_js.dyn_into::<web_sys::Response>().map_err(js_err)?;

	Response::from_web_sys(resp).await
}

impl TryInto<web_sys::Request> for Request {
	type Error = BevyError;

	fn try_into(self) -> Result<web_sys::Request, Self::Error> {
		let init = web_sys::RequestInit::new();
		init.set_method(self.parts.method.as_str());
		if let Some(body) = &self.body {
			init.set_body(&js_sys::Uint8Array::from(body.as_ref()));
		}
		let url = self.parts.uri.to_string();
		let request = web_sys::Request::new_with_str_and_init(&url, &init)
			.map_err(js_err)?;

		for (name, value) in self.parts.headers.iter() {
			let name_str = name.as_str();
			let value_str = value.to_str().map_err(|e| {
				bevyhow!("Failed to set header {}: {}", name_str, e)
			})?;
			request.headers().set(name_str, value_str).map_err(js_err)?;
		}
		Ok(request)
	}
}




impl Response {
	pub async fn from_web_sys(resp: web_sys::Response) -> Result<Self> {
		// Status
		let status = StatusCode::from_u16(resp.status() as u16)
			.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

		// Headers
		let mut headers = http::HeaderMap::new();
		let headers_iter = resp.headers();
		let js_iter = js_sys::try_iter(&headers_iter)
			.map_err(js_err)?
			.ok_or_else(|| bevyhow!("no iterator"))?;
		for entry in js_iter {
			let entry = entry.map_err(js_err)?;
			let arr = js_sys::Array::from(&entry);
			if arr.length() == 2 {
				let key = arr.get(0).as_string().unwrap_or_default();
				let value = arr.get(1).as_string().unwrap_or_default();
				if let (Ok(header_name), Ok(header_value)) = (
					HeaderName::from_bytes(key.as_bytes()),
					HeaderValue::from_str(&value),
				) {
					headers.insert(header_name, header_value);
				}
			}
		}

		let js_array_buffer = wasm_bindgen_futures::JsFuture::from(
			resp.array_buffer().map_err(js_err)?,
		)
		.await
		.map_err(js_err)?;
		let array_buffer = js_sys::Uint8Array::new(&js_array_buffer);
		let mut body = vec![0; array_buffer.length() as usize];
		array_buffer.copy_to(&mut body);
		let bytes = Bytes::from(body);

		let mut builder = http::Response::builder();
		builder = builder.status(status);
		for (key, value) in headers.iter() {
			builder = builder.header(key, value);
		}
		let http_response = builder.body(bytes.clone())?;
		let (parts, body) = http_response.into_parts();
		Ok(Response {
			parts,
			body: body.into(),
		})
	}
}

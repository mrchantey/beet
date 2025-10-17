use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use http::HeaderValue;
use http::StatusCode;
use http::header::HeaderName;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;


pub(crate) async fn send_wasm(request: Request) -> Result<Response> {
	let request: web_sys::Request = request.try_into()?;
	let window = web_sys::window().ok_or_else(|| bevyhow!("No window"))?;
	let promise = window.fetch_with_request(&request);
	let resp_js = wasm_bindgen_futures::JsFuture::from(promise)
		.await
		.map_jserr()?;
	let resp = resp_js.dyn_into::<web_sys::Response>().map_jserr()?;

	Response::from_web_sys(resp).await
}

impl TryInto<web_sys::Request> for Request {
	type Error = BevyError;

	fn try_into(self) -> Result<web_sys::Request, Self::Error> {
		let init = web_sys::RequestInit::new();
		init.set_method(self.parts.method.as_str());

		match &self.body {
			Body::Bytes(bytes) if !bytes.is_empty() => {
				init.set_body(&js_sys::Uint8Array::from(bytes.as_ref()));
			}
			Body::Stream(_) => {
				let stream = create_readable_stream_from_body(self.body)?;
				init.set_body(&stream);
			}
			Body::Bytes(_) => {
				// Empty bytes, no body to set
			}
		}

		let url = self.parts.uri.to_string();
		let request =
			web_sys::Request::new_with_str_and_init(&url, &init).map_jserr()?;

		for (name, value) in self.parts.headers.iter() {
			let name_str = name.as_str();
			let value_str = value.to_str().map_err(|e| {
				bevyhow!("Failed to set header {}: {}", name_str, e)
			})?;
			request.headers().set(name_str, value_str).map_jserr()?;
		}
		Ok(request)
	}
}

fn create_readable_stream_from_body(
	body: Body,
) -> Result<web_sys::ReadableStream> {
	use std::cell::RefCell;
	use std::rc::Rc;
	use wasm_bindgen::prelude::*;
	use wasm_bindgen_futures::spawn_local;

	let body_rc = Rc::new(RefCell::new(body));

	let start = {
		let body_rc = body_rc.clone();
		Closure::wrap(Box::new(
			move |controller: web_sys::ReadableStreamDefaultController| {
				let body_rc = body_rc.clone();
				spawn_local(async move {
					let mut body_ref = body_rc.borrow_mut();
					while let Ok(Some(chunk)) = body_ref.next().await {
						let uint8_array =
							js_sys::Uint8Array::from(chunk.as_ref());
						if controller.enqueue_with_chunk(&uint8_array).is_err()
						{
							break;
						}
					}
					let _ = controller.close();
				});
			},
		)
			as Box<dyn FnMut(web_sys::ReadableStreamDefaultController)>)
	};

	let underlying_source = js_sys::Object::new();
	js_sys::Reflect::set(
		&underlying_source,
		&JsValue::from_str("start"),
		start.as_ref().unchecked_ref(),
	)
	.map_jserr()?;

	start.forget(); // Prevent the closure from being dropped

	let stream =
		web_sys::ReadableStream::new_with_underlying_source(&underlying_source)
			.map_jserr()?;

	Ok(stream)
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
			.map_jserr()?
			.ok_or_else(|| bevyhow!("no iterator"))?;
		for entry in js_iter {
			let entry = entry.map_jserr()?;
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

		let is_bytes = headers
			.get("content-length")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<u64>().ok())
			.map_or(false, |val| val <= Body::MAX_BUFFER_SIZE as u64);

		let body: Body = if is_bytes {
			// body is bytes
			let js_array_buffer = wasm_bindgen_futures::JsFuture::from(
				resp.array_buffer().map_jserr()?,
			)
			.await
			.map_jserr()?;
			let array_buffer = js_sys::Uint8Array::new(&js_array_buffer);
			let mut bytes_vec = vec![0; array_buffer.length() as usize];
			array_buffer.copy_to(&mut bytes_vec);
			Body::Bytes(Bytes::from(bytes_vec))
		} else {
			// body is stream
			use bytes::Bytes;
			use futures::stream;
			use js_sys::Uint8Array;
			use wasm_bindgen::prelude::*;
			use wasm_bindgen_futures::JsFuture;
			use web_sys::ReadableStreamDefaultReader;
			let body = resp.body().ok_or_else(|| bevyhow!("No body"))?;

			let reader: ReadableStreamDefaultReader =
				body.get_reader().dyn_into().unwrap();

			let byte_stream = stream::unfold(reader, |reader| async move {
				let chunk = JsFuture::from(reader.read()).await.ok()?;
				let done =
					js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))
						.ok()?
						.as_bool()?;
				if done {
					return None;
				}

				let value =
					js_sys::Reflect::get(&chunk, &JsValue::from_str("value"))
						.ok()?;
				let bytes = Uint8Array::new(&value).to_vec();
				Some((Ok(Bytes::from(bytes)), reader))
			});

			Body::Stream(SendWrapper::new(Box::pin(byte_stream)))
		};

		let mut builder = http::Response::builder().status(status);
		for (key, value) in headers.iter() {
			builder = builder.header(key, value);
		}
		Ok(builder.body(body)?.into())
	}
}

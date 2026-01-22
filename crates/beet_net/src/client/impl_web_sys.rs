use beet_core::prelude::*;
use bytes::Bytes;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
	/// Global fetch function available in both browsers and Deno
	#[wasm_bindgen(js_name = fetch)]
	fn global_fetch(input: &web_sys::Request) -> js_sys::Promise;
}

pub(super) async fn send_wasm(request: Request) -> Result<Response> {
	let request: web_sys::Request = into_request(request)?;
	let promise = global_fetch(&request);
	let res = wasm_bindgen_futures::JsFuture::from(promise)
		.await
		.map_jserr()?;
	let res = res.dyn_into::<web_sys::Response>().map_jserr()?;

	into_response(res).await
}


fn into_request(req: Request) -> Result<web_sys::Request> {
	let init = web_sys::RequestInit::new();
	let method_str = req.method().to_string().to_uppercase();
	init.set_method(&method_str);

	let (parts, body) = req.into_parts();
	match &body {
		Body::Bytes(bytes) if !bytes.is_empty() => {
			init.set_body(&js_sys::Uint8Array::from(bytes.as_ref()));
		}
		Body::Stream(_) => {
			let stream = create_readable_stream_from_body(body)?;
			init.set_body(&stream);
		}
		Body::Bytes(_) => {
			// Empty bytes, no body to set
		}
	}

	let url = parts.uri().to_string();
	let request =
		web_sys::Request::new_with_str_and_init(&url, &init).map_jserr()?;

	// Set headers from our multimap
	for (name, values) in parts.headers().iter_all() {
		for value in values {
			request.headers().set(name, value).map_jserr()?;
		}
	}
	Ok(request)
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


async fn into_response(res: web_sys::Response) -> Result<Response> {
	let status = StatusCode::from_http_raw(res.status() as u16);

	// Build ResponseParts with headers
	let mut parts = ResponseParts::new(status);

	let headers_iter = res.headers();
	let js_iter = js_sys::try_iter(&headers_iter)
		.map_jserr()?
		.ok_or_else(|| bevyhow!("no iterator"))?;
	for entry in js_iter {
		let entry = entry.map_jserr()?;
		let arr = js_sys::Array::from(&entry);
		if arr.length() == 2 {
			let key = arr.get(0).as_string().unwrap_or_default();
			let value = arr.get(1).as_string().unwrap_or_default();
			parts.parts_mut().insert_header(key.to_lowercase(), value);
		}
	}

	// Check if this is an SSE response which must always be streamed
	let is_event_stream = parts
		.get_header("content-type")
		.map_or(false, |ct| ct.contains("text/event-stream"));

	let is_bytes = !is_event_stream
		&& parts
			.get_header("content-length")
			.and_then(|val| val.parse::<u64>().ok())
			.map_or(false, |val| val <= Body::MAX_BUFFER_SIZE as u64);

	let body: Body = if is_bytes {
		// body is bytes
		let js_array_buffer = wasm_bindgen_futures::JsFuture::from(
			res.array_buffer().map_jserr()?,
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
		let body = res.body().ok_or_else(|| bevyhow!("No body"))?;

		let reader: ReadableStreamDefaultReader =
			body.get_reader().dyn_into().unwrap();

		let byte_stream = stream::unfold(reader, |reader| async move {
			let chunk = JsFuture::from(reader.read()).await.ok()?;
			let done = js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))
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

	Ok(Response::from_parts(parts, Bytes::new()).with_body(body))
}

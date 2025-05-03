use super::*;
use sweet_utils::utils::*;
use wasm_bindgen::JsCast;





impl super::Request {
	pub async fn send(self) -> Result<Response> {
		let request = web_sys::Request::new_with_str_and_init(self.url.as_str(), &{
			let init = web_sys::RequestInit::new();
			init.set_method(&self.method.to_string());
			if let Some(body) = &self.body {
				init.set_body(&js_sys::Uint8Array::from(body.as_slice()));
			}
			init
		})
		.map_err(Error::network)?;

		for (name, value) in &self.headers {
			request
				.headers()
				.set(
					name.as_str(),
					value.to_str().map_err(|e| {
						Error::Serialization(format!(
							"Failed to set header {}: {}",
							name, e
						))
					})?,
				)
				.map_err(Error::network)?;
		}

		let promise = web_sys::window()
			.ok_or_else(|| Error::NetworkError("No window".to_string()))?
			.fetch_with_request(&request);

		let response = wasm_bindgen_futures::JsFuture::from(promise)
			.await
			.map_err(Error::network)?
			.dyn_into::<web_sys::Response>()
			.map_err(Error::network)?;

		Ok(Response::new(response))
	}
}



impl ResponseInner for web_sys::Response {
	fn status_code(&self) -> StatusCode {
		let status = self.status();
		StatusCode::from_u16(status as u16)
			.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
	}
	async fn bytes(self) -> Result<Vec<u8>> {
		self.blob()
			.map_err(Error::network)?
			.xmap(wasm_bindgen_futures::JsFuture::from)
			.await
			.map_err(Error::network)?
			.xmap(|blob| blob.dyn_into::<web_sys::Blob>())
			.map_err(Error::network)?
			.xmap(|blob| blob.array_buffer())
			.xmap(wasm_bindgen_futures::JsFuture::from)
			.await
			.map_err(Error::network)?
			.as_ref()
			.xmap(js_sys::Uint8Array::new)
			.xmap(|arr| {
				let mut vec = vec![0; arr.length() as usize];
				arr.copy_to(&mut vec);
				Ok(vec)
			})
	}
	async fn text(self) -> Result<String> {
		web_sys::Response::text(&self)
			.map_err(Error::network)?
			.xmap(wasm_bindgen_futures::JsFuture::from)
			.await
			.map_err(Error::network)?
			.as_string()
			.ok_or_else(|| {
				Error::NetworkError(
					"Failed to convert response to string".to_string(),
				)
			})
	}
}

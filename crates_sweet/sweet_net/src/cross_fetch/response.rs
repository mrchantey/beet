use super::*;
use http::StatusCode;
use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
type ResponseTy = web_sys::Response;
#[cfg(not(target_arch = "wasm32"))]
type ResponseTy = reqwest::Response;

#[derive(Debug)]
pub struct Response {
	pub inner: ResponseTy,
}

pub trait ResponseInner: Sized {
	fn status_code(&self) -> StatusCode;
	async fn bytes(self) -> Result<Vec<u8>>;
	async fn text(self) -> Result<String>;

	async fn body<T: DeserializeOwned>(self) -> Result<T> {
		let bytes = self.bytes().await?;

		serde_json::from_slice(&bytes).map_err(|e| {
			Error::Deserialization(format!("Failed to deserialize body: {}", e))
		})
	}
	/// Becomes an error if the response is not 2xx
	fn into_result(self) -> Result<Self> {
		if self.status_code().is_success() {
			Ok(self)
		} else {
			Err(Error::ResponseNotOk(self.status_code()))
		}
	}
}


impl Response {
	pub fn new(inner: ResponseTy) -> Self { Self { inner } }
}

impl ResponseInner for Response {
	fn status_code(&self) -> StatusCode { self.inner.status_code() }
	async fn bytes(self) -> Result<Vec<u8>> {
		ResponseInner::bytes(self.inner).await
	}
	async fn text(self) -> Result<String> {
		self.inner
			.text()
			.await
			.map_err(|e| Error::NetworkError(e.to_string()))
	}
}





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet_test::as_sweet::*;
	use sweet_utils::utils::PipelineTarget;

	const HTTPBIN: &str = "https://httpbin.org";

	#[derive(Debug, PartialEq, serde::Deserialize)]
	struct Res {
		data: Body,
	}
	#[derive(Debug, PartialEq, serde::Deserialize)]
	struct Body {
		foo: String,
	}

	#[sweet_test::test]
	async fn works() {
		Request::new(format!("{HTTPBIN}/post"))
			.method(HttpMethod::Post)
			.body(&serde_json::json!({"foo": "bar"}))
			.unwrap()
			.send()
			.await
			.unwrap()
			.body::<serde_json::Value>()
			.await
			.unwrap()
			.xmap(|value| value["json"]["foo"].as_str().unwrap().to_string())
			.xpect()
			.to_be("bar");
	}
}

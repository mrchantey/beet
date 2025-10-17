use crate::prelude::*;
use beet_core::prelude::*;
use reqwest::Client;
use reqwest::RequestBuilder;
use std::sync::LazyLock;


pub async fn send_reqwest(req: Request) -> Result<Response> {
	static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());

	#[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
	{
		if req.parts.uri.scheme_str() == Some("https") {
			beet_core::bevybail!(
				"Please enable either `beet/rustls-tls` or `beet/native-tls` feature to use HTTPS requests."
			);
		}
	}
	let req = req.try_into_reqwest().await?;
	let res = RequestBuilder::from_parts(REQWEST_CLIENT.clone(), req)
		.send()
		.await?;
	let res = Response::try_from_reqwest(res).await?;
	Ok(res)
}

impl Request {
	pub async fn try_into_reqwest(self) -> Result<reqwest::Request> {
		match self.body {
			Body::Bytes(bytes) => {
				let http_req = http::Request::from_parts(self.parts, bytes);
				http_req.try_into().map_err(BevyError::from)
			}
			Body::Stream(stream) => {
				use futures::TryStreamExt;
				use reqwest::Body as ReqwestBody;

				let stream_inner = stream.take();
				let reqwest_body =
					ReqwestBody::wrap_stream(stream_inner.map_err(|e| {
						std::io::Error::new(
							std::io::ErrorKind::Other,
							format!("{}", e),
						)
					}));

				let mut builder = reqwest::Request::new(
					self.parts.method.clone(),
					self.parts
						.uri
						.to_string()
						.parse()
						.map_err(BevyError::from)?,
				);

				*builder.headers_mut() = self.parts.headers;
				*builder.body_mut() = Some(reqwest_body);

				Ok(builder)
			}
		}
	}
}

impl Response {
	pub async fn try_from_reqwest(mut res: reqwest::Response) -> Result<Self> {
		let mut builder = http::Response::builder().status(res.status());
		let headers = builder.headers_mut().unwrap();
		std::mem::swap(headers, res.headers_mut());

		let headers = res.headers();
		let is_bytes = headers
			.get("content-length")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<u64>().ok())
			.map_or(false, |val| val <= Body::MAX_BUFFER_SIZE as u64);

		let body = if is_bytes {
			Body::Bytes(res.bytes().await?.into())
		} else {
			use futures::TryStreamExt;
			use send_wrapper::SendWrapper;

			Body::Stream(SendWrapper::new(Box::pin(
				res.bytes_stream().map_err(BevyError::from),
			)))
		};

		Ok(builder.body(body)?.into())
	}
}

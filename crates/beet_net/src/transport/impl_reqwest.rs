use crate::prelude::*;
use bevy::prelude::*;
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
	let req = req.try_into()?;
	let res = RequestBuilder::from_parts(REQWEST_CLIENT.clone(), req)
		.send()
		.await?;
	let res = Response::try_from_reqwest(res).await?;
	Ok(res)
}

impl TryInto<reqwest::Request> for Request {
	type Error = reqwest::Error;

	fn try_into(self) -> Result<reqwest::Request, Self::Error> {
		self.into_http_request().try_into()
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

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
			bevybail!(
				"Please enable either the `rustls-tls` or `native-tls` feature to use HTTPS requests."
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
		let mut builder = http::Response::builder();
		if let Some(headers) = builder.headers_mut() {
			std::mem::swap(headers, res.headers_mut());
		}
		let res = builder.status(res.status()).body(res.bytes().await?)?;
		Ok(res.into())
	}
}

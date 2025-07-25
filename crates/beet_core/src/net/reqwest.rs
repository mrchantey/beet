use crate::prelude::*;
use bevy::prelude::*;
use reqwest::Client;
use reqwest::RequestBuilder;
use std::sync::LazyLock;

static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());

/// A wrapper around the reqwest client. Calling `Client::new()` creates
/// a new client, which is expensive. This wrapper creates a single
/// static client and returns a reference to it.
pub struct ReqwestClient;

impl ReqwestClient {
	/// Returns a reference to the static reqwest client.
	pub fn client() -> &'static Client { &*REQWEST_CLIENT }

	pub async fn send(req: Request) -> Result<Response> {
		#[cfg(not(any(feature = "rustls", feature = "native-tls")))]
		{
			if req.parts.uri.scheme_str() == Some("https") {
				bevybail!(
					"Please enable either the `rustls` or `native-tls` feature to use HTTPS requests."
				);
			}
		}
		let req = req.try_into()?;
		let res = RequestBuilder::from_parts(Self::client().clone(), req)
			.send()
			.await?;
		let res = Response::try_from_reqwest(res).await?;
		Ok(res)
	}
}


impl TryInto<reqwest::Request> for Request {
	type Error = reqwest::Error;

	fn try_into(self) -> Result<reqwest::Request, Self::Error> {
		self.into_http_request().try_into()
	}
}


impl Request {
	pub async fn send(self) -> Result<Response> {
		ReqwestClient::send(self).await
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



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	#[ignore = "flaky example.com"]
	async fn works() {
		ReqwestClient::client()
			.get("https://example.com")
			.send()
			.await
			.xpect()
			.to_be_ok();
	}
}

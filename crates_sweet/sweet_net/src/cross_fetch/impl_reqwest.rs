use super::*;
use crate::prelude::*;
use sweet_utils::utils::*;





impl super::Request {
	pub async fn send(self) -> Result<Response> {
		ReqwestClient::client()
			.request(self.method.into(), self.url)
			.headers(self.headers)
			.xmap(|mut req| {
				if let Some(body) = self.body {
					req = req.body(body.clone());
				}
				if let Some(timeout) = self.timeout {
					req = req.timeout(timeout);
				}
				req
			})
			.send()
			.await?
			.xinto::<Response>()
			.xok()
	}
}


impl ResponseInner for reqwest::Response {
	fn status_code(&self) -> StatusCode { self.status() }
	async fn bytes(self) -> Result<Vec<u8>> {
		self.bytes()
			.await
			.map_err(|e| Error::NetworkError(e.to_string()))?
			.to_vec()
			.xok()
	}
	async fn text(self) -> Result<String> {
		reqwest::Response::text(self).await?.xok()
	}
}

impl From<reqwest::Response> for Response {
	fn from(res: reqwest::Response) -> Self { Response::new(res) }
}


impl From<reqwest::Error> for Error {
	fn from(err: reqwest::Error) -> Self {
		Error::NetworkError(err.to_string())
	}
}

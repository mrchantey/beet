use crate::dom::PageProvider;
use base64::prelude::*;
use beet_core::bevybail;
use beet_core::utils::SendBoxedFuture;
use bevy::prelude::*;




pub struct WebdriverPage {
	client: fantoccini::Client,
}
impl WebdriverPage {
	pub fn new(client: fantoccini::Client) -> Self { Self { client } }
}

impl PageProvider for WebdriverPage {
	fn visit(&self, url: &str) -> SendBoxedFuture<Result> {
		let client = self.client.clone();
		let url = url.to_string();
		Box::pin(async move {
			client.goto(&url).await?;
			Ok(())
		})
	}



	fn export_pdf(&self) -> SendBoxedFuture<Result<bytes::Bytes>> {
		let client = self.client.clone();
		Box::pin(async move {
			let response = client
				.execute(
					"return (async function() {
						const pdf = await window.print();
						return pdf;
					})()",
					vec![],
				)
				.await?;

			if let Some(data) = response.as_str() {
				let pdf_bytes = match BASE64_STANDARD.decode(data) {
					Ok(bytes) => bytes,
					Err(e) => bevybail!("Base64 decode error: {}", e),
				};
				Ok(pdf_bytes.into())
			} else {
				bevybail!("No PDF data in response")
			}
		})
	}

	fn current_url(&self) -> SendBoxedFuture<Result<String>> {
		let client = self.client.clone();
		Box::pin(async move { Ok(client.current_url().await?.to_string()) })
	}

	fn eval_async(
		&self,
		_script: &str,
		_args: Vec<serde_json::Value>,
	) -> SendBoxedFuture<Result<serde_json::Value>> {
		let _client = self.client.clone();
		Box::pin(async move {
			unimplemented!()
			//Ok(client.execute_async(script, args))
		})
	}
}

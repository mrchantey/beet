use crate::prelude::*;
use base64::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde_json::json;

#[derive(Clone)]
pub struct ChromePage {
	connection: DevToolsConnection,
}

impl ChromePage {
	pub async fn connect() -> Result<Self> {
		Self::new(DevToolsConnection::connect().await?).xok()
	}
	pub fn new(connection: DevToolsConnection) -> Self { Self { connection } }

	pub async fn visit(&mut self, url: impl AsRef<str>) -> Result<&mut Self> {
		let url = url.as_ref();
		self.connection
			.send(json!({
				"method": "Page.navigate",
				"params": { "url": url }
			}))
			.await?;
		Ok(self)
	}

	pub async fn export_pdf(&mut self) -> Result<Vec<u8>> {
		// how to wait for full page load?
		time_ext::sleep_secs(5).await;

		let response = self
			.connection
			.send_with_backoff(json!({
				"method": "Page.printToPDF",
				"params": {
					"displayHeaderFooter": false,
					"printBackground": true,
					"marginTop": 0,
					"marginBottom": 0,
					"marginLeft": 0,
					"marginRight": 0
				}
			}))
			.await?;

		if let Some(data) = response["result"]["data"].as_str() {
			let pdf_bytes = match BASE64_STANDARD.decode(data) {
				Ok(bytes) => bytes,
				Err(e) => bevybail!("Base64 decode error: {}", e),
			};
			Ok(pdf_bytes)
		} else {
			bevybail!("No PDF data in response")
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	// #[ignore = "requires Chrome DevTools"]
	async fn works() {
		let devtools = DevToolsProcess::spawn().await.unwrap();
		let url = devtools.url();
		let _ = DevToolsProcess::await_up(url).await.unwrap();

		// let bytes = export_pdf("https://google.com").await.unwrap();
		// let _devtools = ChromeDevTools::spawn().await.unwrap();
		let bytes = ChromePage::connect()
			.await
			.unwrap()
			.visit("https://google.com")
			// .visit("https://beetstack.dev")
			.await
			.unwrap()
			.export_pdf()
			.await
			.unwrap();

		// let bytes = export_pdf("https://beetstack.dev").await.unwrap();
		bytes.len().xpect_greater_than(100);
		devtools.kill().await.unwrap();
	}
}

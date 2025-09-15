use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use serde_json::json;

#[derive(Clone)]
pub struct DevToolsPage {
	client: DevToolsConnection,
}

impl DevToolsPage {
	pub fn new(conn: DevToolsConnection) -> Self { Self { client: conn } }
}

impl PageProvider for DevToolsPage {
	fn visit(&self, url: &str) -> SendBoxedFuture<Result<()>> {
		let client = self.client.clone();
		let args = json!({
			"method": "Page.navigate",
			"params": { "url": url }
		});
		Box::pin(async move {
			client.send(args).await?;
			Ok(())
		})
	}

	fn current_url(&self) -> SendBoxedFuture<Result<String>> {
		let client = self.client.clone();
		let args = json!({
			"method": "Runtime.evaluate",
			"params": {
				"expression": "window.location.href",
				"returnByValue": true
			}
		});
		Box::pin(async move {
			let response = client.send(args).await?;
			if let Some(url) = response["result"]["result"]["value"].as_str() {
				Ok(url.to_string())
			} else {
				bevybail!("No URL in response")
			}
		})
	}



	fn export_pdf(&self) -> SendBoxedFuture<Result<Bytes>> {
		// how to wait for full page load?
		let client = self.client.clone();
		let args = json!({
			"method": "Page.printToPDF",
			"params": {
				"displayHeaderFooter": false,
				"printBackground": true,
				"marginTop": 0,
				"marginBottom": 0,
				"marginLeft": 0,
				"marginRight": 0
			}
		});

		Box::pin(async move {
			time_ext::sleep_secs(5).await;

			let response = client.send(args).await?;

			if let Some(data) = response["result"]["data"].as_str() {
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

	fn eval_async(
		&self,
		script: &str,
		args: Vec<serde_json::Value>,
	) -> SendBoxedFuture<Result<serde_json::Value>> {
		let client = self.client.clone();
		let func_args: Vec<serde_json::Value> =
			args.into_iter().map(|a| json!({ "value": a })).collect();

		let params = json!({
			"method": "Runtime.callFunctionOn",
			"params": {
				"functionDeclaration": script,
				"arguments": func_args,
				"awaitPromise": true,
				"returnByValue": true
			}
		});

		Box::pin(async move {
			let response = client.send(params).await?;

			// Try to return the value returned by the function (when returned by value)
			if response["result"]["result"].get("value").is_some() {
				Ok(response["result"]["result"]["value"].clone())
			} else if response["result"]["result"].is_object() {
				// Fallback: return the whole result object
				Ok(response["result"]["result"].clone())
			} else {
				bevybail!("No value in Runtime.callFunctionOn response")
			}
		})
	}
}

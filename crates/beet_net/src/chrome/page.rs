use crate::chrome::net_backoff;
use base64::Engine;
use beet_core::bevybail;
use beet_rsx::as_beet::ValueExt;
use beet_utils::time_ext;
use bevy::prelude::*;
use futures::SinkExt;
use futures::StreamExt;
use serde_json::Value;
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Page {
	ws: Socket,
	next_id: usize,
}


impl Page {
	pub fn new(ws: Socket) -> Self { Self { ws, next_id: 0 } }

	fn next_id(&mut self) -> usize {
		let id = self.next_id;
		self.next_id += 1;
		id
	}

	/// send a message with the id inserted, awaiting a matching response
	async fn send(&mut self, mut body: Value) -> Result<Value> {
		let id = self.next_id();
		body.set_field("id", Value::Number(id.into()))?;
		self.ws.send(Message::Text(body.to_string().into())).await?;
		self.await_response(id).await
	}

	async fn send_with_backoff(&mut self, body: Value) -> Result<Value> {
		for duration in net_backoff() {
			match (self.send(body.clone()).await, duration) {
				(Ok(value), _) => return Ok(value),
				(Err(_), Some(duration)) => {
					println!(
						"Backoff Error, retrying in {}ms",
						duration.as_millis()
					);
					time_ext::sleep(duration).await;
				}
				(Err(err), None) => {
					bevybail!("failed to connect to devtools: {}", err)
				}
			}
		}
		unreachable!()
	}

	/// await a text response with the corresponding id, discarding
	/// all other messages
	async fn await_response(&mut self, id: usize) -> Result<Value> {
		while let Some(msg) = self.ws.next().await {
			match msg {
				Ok(Message::Text(text)) => {
					let response = serde_json::from_str::<Value>(&text)?;
					if response["id"] != id {
						println!("unhandled message: {:#?}", response);
						continue;
					}
					if response["error"].is_object() {
						bevybail!(
							"Page Error: {}",
							response["error"]["message"]
						);
					} else {
						return Ok(response);
					}
				}
				_ => {}
			}
		}
		bevybail!("WebSocket connection closed before matching id returned")
	}

	pub async fn visit(&mut self, url: impl AsRef<str>) -> Result<()> {
		self.send(json!({
			"method": "Page.navigate",
			"params": { "url": url.as_ref() }
		}))
		.await?;
		Ok(())
	}

	pub async fn export_pdf(&mut self) -> Result<Vec<u8>> {
		// how to wait for full page load?
		time_ext::sleep_secs(5).await;

		let response = self
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
			let pdf_bytes =
				match base64::engine::general_purpose::STANDARD.decode(data) {
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
	#[ignore = "requires Chrome DevTools"]
	async fn works() {
		// let bytes = export_pdf("https://google.com").await.unwrap();
		let devtools = ChromeDevTools::connect().await.unwrap();
		let bytes = devtools
			.visit("https://google.com")
			// .visit("https://beetstack.dev")
			.await
			.unwrap()
			.export_pdf()
			.await
			.unwrap();

		// let bytes = export_pdf("https://beetstack.dev").await.unwrap();
		bytes.len().xpect_greater_than(100);
	}
}

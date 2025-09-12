use super::DEFAULT_PORT;
use crate::chrome::ChromeDevTools;
use base64::prelude::*;
use beet_core::bevybail;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use serde_json::json;

pub struct ChromePage {
	/// url that this page is currently visiting
	url: String,
	socket: Socket,
	// each socket message is assigned an id, to be matched with the received message
	next_msg_id: usize,
}

const DEFAULT_URL: &str = "about:blank";
impl ChromePage {
	pub async fn connect() -> Result<Self> {
		let url = Url::parse(&format!("http://127.0.0.1:{DEFAULT_PORT}"))?;
		Self::connect_with_url(&url).await
	}

	pub async fn connect_with_url(dev_tools_url: &Url) -> Result<Self> {
		let socket_url =
			ChromeDevTools::socket_url(dev_tools_url, "page").await?;
		let socket = Socket::connect(socket_url).await?;

		let this = Self::new(socket, DEFAULT_URL);
		// this.visit(DEFAULT_URL).await?;
		Ok(this)
	}

	pub fn new(socket: Socket, url: impl AsRef<str>) -> Self {
		Self {
			url: url.as_ref().to_string(),
			socket,
			next_msg_id: 0,
		}
	}

	fn next_id(&mut self) -> usize {
		let id = self.next_msg_id;
		self.next_msg_id += 1;
		id
	}

	pub async fn visit(&mut self, url: impl AsRef<str>) -> Result<&mut Self> {
		let url = url.as_ref();
		self.send(json!({
			"method": "Page.navigate",
			"params": { "url": url }
		}))
		.await?;
		self.url = url.to_string();
		Ok(self)
	}


	/// send a message with the id inserted, awaiting a matching response
	async fn send(&mut self, mut body: Value) -> Result<Value> {
		let id = self.next_id();
		body.set_field("id", Value::Number(id.into()))?;
		self.socket
			.send(Message::Text(body.to_string().into()))
			.await?;
		self.await_response(id).await
	}

	async fn send_with_backoff(&mut self, body: Value) -> Result<Value> {
		while let Some(frame) = Backoff::default().stream().next().await {
			match self.send(body.clone()).await {
				Ok(val) => return Ok(val),
				Err(err) if frame.is_final() => return Err(err),
				_ => {
					// discard error on retry
				}
			}
		}
		unreachable!("returned error on final")
	}

	/// await a text response with the corresponding id, discarding
	/// all other messages
	async fn await_response(&mut self, id: usize) -> Result<Value> {
		while let Some(msg) = self.socket.next().await {
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
		// let bytes = export_pdf("https://google.com").await.unwrap();
		let _devtools = ChromeDevTools::spawn().await.unwrap();
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
	}
}

use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::RwLock;

pub(super) const DEFAULT_DEVTOOLS_PORT: u16 = 9222;
pub(super) const DEFAULT_DEVTOOLS_URL: &str = "http://127.0.0.1:9222";

#[derive(Clone)]
pub struct DevToolsConnection {
	// each socket message is assigned an id, to be matched with the received message
	next_msg_id: usize,
	/// Url to the dev tools process, ie `http://127.0.0.1:9222`
	url: Url,
	/// Socket to the page
	socket: Arc<RwLock<Socket>>,
}

impl DevToolsConnection {
	pub fn url(&self) -> &Url { &self.url }

	pub async fn connect() -> Result<Self> {
		Self::connect_with_url(DEFAULT_DEVTOOLS_URL).await
	}
	/// Connect to chrome dev tools at the provided url
	pub async fn connect_with_url(url: impl AsRef<str>) -> Result<Self> {
		let url = url.as_ref();
		Self::await_up(url).await?;
		let socket_url = Self::socket_url(url, "page").await?;
		let socket = Socket::connect(socket_url).await?;

		Ok(Self {
			url: url.try_into()?,
			socket: Arc::new(RwLock::new(socket)),
			next_msg_id: 0,
		})
	}


	fn next_id(&mut self) -> usize {
		let id = self.next_msg_id;
		self.next_msg_id += 1;
		id
	}

	/// send a message with the id inserted, awaiting a matching response
	pub async fn send(&mut self, mut body: Value) -> Result<Value> {
		let id = self.next_id();
		body.set_field("id", Value::Number(id.into()))?;
		self.socket
			.write()
			.await
			.send(Message::Text(body.to_string().into()))
			.await?;
		self.await_response(id).await
	}
	pub async fn send_with_backoff(&mut self, body: Value) -> Result<Value> {
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
		while let Some(msg) = self.socket.write().await.next().await {
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
	pub async fn spawn() -> Result<Child> {
		Self::spawn_with_port(DEFAULT_DEVTOOLS_PORT).await
	}

	pub async fn spawn_with_port(port: u16) -> Result<Child> {
		let cmd = vec![
			"chromium".into(),
			"--headless".into(),
			"--disable-gpu".into(),
			// "--remote-debugging-port={port}".into(),
			// "--auto-open-devtools-for-tabs".into(),
			// "--user-data-dir=$(mktemp -d)".into(),
			format!("--remote-debugging-port={port}"),
		];
		Command::new("nix-shell")
			.arg("-p")
			.arg("chromium")
			.arg("--run")
			.arg(format!(r#"bash -lc "{}""#, cmd.join(" ")))
			.kill_on_drop(true)
			.spawn()?
			.xok()
	}
	/// waits for the first Ok response from the dev tools url
	pub async fn await_up(url: impl AsRef<str>) -> Result {
		let url = url.as_ref();
		let _ = Backoff::default()
			.with_max(Duration::from_millis(500))
			.with_max_attempts(20)
			.retry_async(async |frame| {
				let result =
					Request::get(url).send().await?.into_result().await;
				if !result.is_ok() {
					println!("Awaiting Chrome Devtools Connection: {}", frame);
				}
				result
			})
			.await?;
		Ok(())
	}
	/// Get the socket url using a short exponential backoff, assumes
	/// process is already up
	pub async fn socket_url(url: &str, conn_type: &str) -> Result<String> {
		let url = format!("{url}/json");
		let res = Backoff::default()
			.retry_async(async |_| {
				Request::get(&url).send().await?.into_result().await
			})
			.await?;
		match res
			.json::<Value>()
			.await?
			.to_array()?
			.iter()
			.find(|e| e.field_str("type").ok() == Some(conn_type))
		{
			Some(entry) => {
				let url = entry.field_str("webSocketDebuggerUrl")?;
				return Ok(url.to_string());
			}
			None => {
				bevybail!(
					"connected to devtools but couldn't find a page target"
				)
			}
		}
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	// #[ignore = "requires Chrome DevTools"]
	async fn works() {
		let mut devtools = DevToolsConnection::spawn().await.unwrap();
		let _conn = DevToolsConnection::connect().await.unwrap();
		devtools.kill().await.unwrap();
	}
}

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::RwLock;

pub struct DevToolsClient {
	conn: DevToolsConnection,
	/// The spawned browser process when requested via ConnectOptions
	process: Option<Child>,
}

impl DevToolsClient {
	pub async fn connect() -> Result<Self> {
		// Use the ConnectOptions defined in `dom::mod.rs`
		Self::connect_with_opts(default()).await
	}
	pub async fn connect_with_opts(opts: ConnectOptions) -> Result<Self> {
		let process = if opts.spawn_process {
			Some(Self::spawn_chromium(&opts).await?)
		} else {
			None
		};

		let url = opts.url();
		Self::await_up(&url).await?;
		let socket_url = Self::socket_url(&url, "page").await?;
		let socket = Socket::connect(socket_url).await?;
		let inner = DevToolsConnection::new(socket);

		Ok(Self {
			conn: inner,
			process,
		})
	}

	pub fn page(&self) -> Page {
		let provider = DevToolsPage::new(self.conn.clone());
		Page::new(provider)
	}


	/// waits for the first Ok response from the dev tools url
	async fn await_up(url: impl AsRef<str>) -> Result {
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
	async fn socket_url(url: &str, conn_type: &str) -> Result<String> {
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
	pub async fn kill(self) -> Result<()> {
		self.conn.kill().await?;
		if let Some(mut process) = self.process {
			process.kill().await?;
		}
		Ok(())
	}
}

#[derive(Clone)]
pub struct DevToolsConnection {
	/// each socket message is assigned an id, to be matched with the received message
	next_msg_id: Arc<AtomicUsize>,
	/// Socket to the page
	socket: Arc<RwLock<Socket>>,
}

impl DevToolsConnection {
	pub fn new(socket: Socket) -> Self {
		Self {
			socket: Arc::new(RwLock::new(socket)),
			next_msg_id: Arc::new(AtomicUsize::new(0)),
		}
	}

	fn next_id(&self) -> usize {
		self.next_msg_id.fetch_add(1, Ordering::SeqCst)
	}

	/// send a message with the id inserted, awaiting a matching response
	pub async fn send(&self, mut body: Value) -> Result<Value> {
		let id = self.next_id();
		body.set_field("id", Value::Number((id as u64).into()))?;
		self.socket
			.write()
			.await
			.send(Message::Text(body.to_string().into()))
			.await?;
		self.await_response(id).await
	}
	pub async fn send_with_backoff(&self, body: Value) -> Result<Value> {
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
	async fn await_response(&self, id: usize) -> Result<Value> {
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

	pub async fn kill(self) -> Result<()> {
		self.socket.write().await.close(None).await?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	// #[ignore = "requires Chrome DevTools"]
	async fn works() {
		let client = DevToolsClient::connect().await.unwrap();
		client.kill().await.unwrap();
	}
}

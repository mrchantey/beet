use beet_core::exports::async_channel;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// Inner shared state so `Session` can be cloned.

struct SessionInner {
	driver_url: String,
	session_id: String,
	socket_url: String,

	/// Next outbound BiDi command id.
	next_id: AtomicUsize,

	/// Pending command responses keyed by id.
	pending: Mutex<HashMap<u64, async_channel::Sender<Value>>>,

	/// Outbound command text frames.
	cmd_tx: async_channel::Sender<String>,
	_cmd_rx: async_channel::Receiver<String>,

	/// Optional writer half so we can close the socket gracefully.
	writer: Mutex<Option<SocketWrite>>,

	/// Event stream (BiDi messages without an id but with a method)
	events_tx: async_channel::Sender<Value>,
	events_rx: async_channel::Receiver<Value>,
}

/// A BiDi WebDriver session (cross platform, wasm friendly).
#[derive(Debug, Clone)]
pub struct Session {
	inner: Arc<SessionInner>,
}

impl std::fmt::Debug for SessionInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SessionInner")
			.field("session_id", &self.session_id)
			.field("socket_url", &self.socket_url)
			.finish()
	}
}

impl Session {
	/// Gracefully delete the webdriver session (classic WebDriver DELETE)
	/// and close the underlying websocket (if this is the last clone holding it).
	pub async fn kill(self) -> Result<()> {
		// Classic WebDriver DELETE
		let url = format!(
			"{}/session/{}",
			self.inner.driver_url, self.inner.session_id
		);
		Request::delete(&url).send().await?.into_result().await?;

		// Try to close the socket (only once)
		if let Some(writer) = self.inner.writer.lock().unwrap().take() {
			// Ignore close errors – session already deleted.
			let _ = writer.close(None).await;
		}
		Ok(())
	}

	/// Access session id.
	pub fn id(&self) -> &str { &self.inner.session_id }

	/// Try to receive the next event (non-blocking).
	pub fn try_event(&self) -> Option<Value> {
		self.inner.events_rx.try_recv().ok()
	}

	/// Send a BiDi command and await the full JSON response (the full object
	/// containing at least "id" and usually "result" or "error").
	pub async fn command(&self, method: &str, params: Value) -> Result<Value> {
		let id = self.inner.next_id.fetch_add(1, Ordering::SeqCst) as u64;

		let (tx, rx) = async_channel::bounded(1);
		{
			let mut pending = self.inner.pending.lock().unwrap();
			pending.insert(id, tx);
		}

		let payload = json!({
			"id": id,
			"method": method,
			"params": params
		});
		let raw = serde_json::to_string(&payload)
			.map_err(|e| bevyhow!("Failed to serialize command: {}", e))?;

		self.inner
			.cmd_tx
			.send(raw)
			.await
			.map_err(|_| bevyhow!("Command channel closed"))?;

		let resp = rx
			.recv()
			.await
			.map_err(|_| bevyhow!("Response channel closed"))?;

		if let Some(err_obj) = resp.get("error") {
			return Err(bevyhow!(
				"BiDi error for method '{}': {}",
				method,
				err_obj
			));
		}
		Ok(resp)
	}

	/// Connect to the BiDi websocket and spawn dispatcher tasks.
	pub async fn connect(
		driver_url: &str,
		session_id: &str,
		socket_url: &str,
	) -> Result<Self> {
		let socket = Socket::connect(socket_url).await?;

		let (read, write) = socket.split();
		let (cmd_tx, cmd_rx) = async_channel::unbounded::<String>();
		let (events_tx, events_rx) = async_channel::unbounded::<Value>();

		let inner = Arc::new(SessionInner {
			driver_url: driver_url.to_string(),
			session_id: session_id.to_string(),
			socket_url: socket_url.to_string(),
			next_id: AtomicUsize::new(1),
			pending: Mutex::new(HashMap::new()),
			cmd_tx,
			_cmd_rx: cmd_rx.clone(),
			writer: Mutex::new(Some(write)),
			events_tx,
			events_rx,
		});

		Self::spawn_writer(inner.clone(), cmd_rx);
		Self::spawn_reader(inner.clone(), read);

		Ok(Self { inner })
	}

	fn spawn_writer(
		inner: Arc<SessionInner>,
		cmd_rx: async_channel::Receiver<String>,
	) {
		IoTaskPool::get()
			.spawn(async move {
				while let Ok(raw) = cmd_rx.recv().await {
					let send_result = {
						let mut guard = inner.writer.lock().unwrap();
						if let Some(writer) = guard.as_mut() {
							writer.send(Message::text(raw)).await
						} else {
							Ok(())
						}
					};
					if send_result.is_err() {
						// Writer gone – stop writer task.
						break;
					}
				}
			})
			.detach();
	}

	fn spawn_reader(inner: Arc<SessionInner>, mut read: SocketRead) {
		IoTaskPool::get()
			.spawn(async move {
				while let Some(item) = read.next().await {
					let Ok(Message::Text(text)) = item else {
						continue;
					};
					let Ok(val) = serde_json::from_str::<Value>(&text) else {
						continue;
					};

					// Response (has id)
					if let Some(id) = val.get("id").and_then(|v| v.as_u64()) {
						let pending = {
							let mut pending_map = inner.pending.lock().unwrap();
							pending_map.remove(&id)
						};
						if let Some(tx) = pending {
							let _ = tx.send(val).await;
						}
						continue;
					}

					// Event (has method, no id)
					if val.get("method").is_some() {
						let _ = inner.events_tx.try_send(val);
					}
				}
			})
			.detach();
	}
}

#[cfg(all(test, feature = "tokio", feature = "webdriver"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use serde_json::json;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		App::default()
			.run_io_task(async move {
				let client = Client::chromium();
				let client =
					ClientProcess::new_with_opts(client.clone()).unwrap();
				let session = client.new_session().await.unwrap();
				// 1. Get browsing contexts
				let tree = session
					.command("browsingContext.getTree", json!({"maxDepth": 0}))
					.await
					.unwrap();

				let contexts = tree["result"]["contexts"]
					.as_array()
					.expect("contexts array missing");
				contexts.is_empty().xmap(|b| !b).xpect_true();
				let context_id = contexts[0]["context"]
					.as_str()
					.expect("context id missing");

				// 2. Navigate
				session
					.command(
						"browsingContext.navigate",
						json!({
							"context": context_id,
							"url": "https://example.com",
							"wait": "complete"
						}),
					)
					.await
					.unwrap();

				// 3. Evaluate heading text
				session
					.command(
						"script.evaluate",
						json!({
							"expression": "document.querySelector('h1')?.textContent",
							"target": { "context": context_id },
							"awaitPromise": true,
							"resultOwnership": "root"
						}),
					)
					.await
					.unwrap()["result"]["result"]["value"]
					.as_str()
					.unwrap()
					.xpect_eq("Example Domain");

				// 4. Click anchor
				session
					.command(
						"script.evaluate",
						json!({
							"expression": "document.querySelector('a')?.click(); 'clicked';",
							"target": { "context": context_id },
							"awaitPromise": true
						}),
					)
					.await
					.unwrap();

				// 5. Query current URL (navigation may or may not change depending on driver timing)
				session
					.command(
						"script.evaluate",
						json!({
							"expression": "location.href",
							"target": { "context": context_id },
							"awaitPromise": true
						}),
					)
					.await
					.unwrap()["result"]["result"]["value"]
					.as_str()
					.unwrap()
					.xpect_eq("https://www.iana.org/help/example-domains");

				// Cleanup
				session.kill().await.unwrap();
				client.kill().await.unwrap();
			})
			.await;
	}
}

use crate::prelude::*;
use crate::sockets::Message;
use crate::sockets::*;
use beet_core::exports::async_channel;
use beet_core::prelude::*;
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

	/// Outbound command text frames, drained by the socket pump. Closing the
	/// channel (see [`Session::kill`]) ends the pump, which closes the socket.
	cmd_tx: async_channel::Sender<String>,

	/// Event subscribers as `(method prefix, sender)` pairs, matched against
	/// each inbound method-bearing frame. A subscriber whose receiver has been
	/// dropped is pruned on the next event.
	subscribers: Mutex<Vec<(String, async_channel::Sender<Value>)>>,
}

/// A BiDi WebDriver session (cross platform, wasm friendly).
///
/// Channel / Task Pattern Overview
/// ===============================
/// The `Session` is a thin, cloneable handle over an internal reference-
/// counted `SessionInner`. Internally we decouple caller futures from
/// the websocket IO using three core pieces:
///
/// 1. Command Channel (`cmd_tx`)
///    Callers invoke `Session::command`, which:
///      - Allocates a monotonically increasing id (`next_id`)
///      - Registers a one‑shot sender in `pending` keyed by that id
///      - Serializes the outbound JSON {id, method, params}
///      - Pushes the raw string onto `cmd_tx`
///
/// 2. Socket Pump (`spawn_pump`)
///    One background task owns the whole socket lifecycle: it connects,
///    splits, drains `cmd_rx` into outbound text frames, and routes inbound
///    frames:
///      - If a message parses and contains an `id`, it is a response.
///        The matching one‑shot sender (if still present) is removed
///        from `pending` and fulfilled with the full JSON object.
///      - If it lacks an `id` but contains `method`, it is an unsolicited
///        event, delivered to every subscriber (see [`Session::events`])
///        whose method prefix matches.
///    The socket's reader/writer halves are thread-bound (`SendWrapper`),
///    so the pump is a single `spawn_local` task: the socket is created and
///    polled on one thread, never sent across the pool (a plain `spawn`
///    migrates between pool threads and panics the `SendWrapper`).
///
/// Error Handling & Backpressure
/// -----------------------------
/// * Each in‑flight command has exactly one awaiting receiver.
/// * Dropping a receiver before fulfillment simply discards the response,
///   because the pending entry is removed only on match.
/// * Event channels are unbounded; dropping a receiver unsubscribes it (the
///   pump prunes closed senders on the next event).
///
/// Concurrency & Safety
/// --------------------
/// * `pending` is guarded by a `Mutex` because operations are short and
///   low contention (only command send / response match).
/// * The socket pump runs on the `IoTaskPool`'s thread-local executor, so an
///   app loop must be ticking it: any running beet app qualifies, including
///   the test runner's own (async `#[beet_core::test]` bodies need no extra
///   `run_io_task_local` wrapper).
///
/// High‑Level Extensions
/// ---------------------
/// Higher constructs (e.g. `Page`, `Element`) compose over `Session` by
/// calling `command` with BiDi methods, interpreting the returned JSON,
/// and introducing richer ergonomics / state tracking.
///
/// Ping / Health
/// -------------
/// A lightweight `ping()` helper issues a benign BiDi round‑trip to
/// validate the full pipeline (id allocation -> writer -> socket ->
/// reader -> pending fulfillment).
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

		// Closing the command channel ends the pump's writer loop, which closes
		// the socket on the pump's own thread (the writer half is thread-bound,
		// so it must not be touched from this caller thread).
		self.inner.cmd_tx.close();
		Ok(())
	}

	/// Access session id.
	pub fn id(&self) -> &str { &self.inner.session_id }

	/// Register a listener for BiDi events whose `method` starts with
	/// `prefix`, eg `"log.entryAdded"` for one event or `"network."` for a
	/// whole module. Registration alone delivers nothing: pair it with
	/// [`Self::subscribe`] so the browser actually emits the events. Dropping
	/// the receiver unsubscribes the listener.
	pub fn events(
		&self,
		prefix: impl Into<String>,
	) -> async_channel::Receiver<Value> {
		let (tx, rx) = async_channel::unbounded();
		self.inner
			.subscribers
			.lock()
			.unwrap()
			.push((prefix.into(), tx));
		rx
	}

	/// Issue `session.subscribe` for the given BiDi event names, optionally
	/// scoped to specific browsing contexts. Listen via [`Self::events`],
	/// registered *before* subscribing so no event slips between the two.
	pub async fn subscribe(
		&self,
		events: &[&str],
		contexts: Option<&[&str]>,
	) -> Result<()> {
		let mut params = json!({ "events": events });
		if let Some(contexts) = contexts {
			params.set_field("contexts", json!(contexts))?;
		}
		self.command("session.subscribe", params).await?;
		Ok(())
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

	/// Cheap liveness / round‑trip check.
	/// performs a simple `browsingContext.getTree`
	///
	/// Success proves:
	/// * id allocation
	/// * writer task operational
	/// * websocket open
	/// * reader task dispatch
	/// * response routed via `pending`
	pub async fn ping(&self) -> Result<()> {
		let _ = self
			.command("browsingContext.getTree", json!({"maxDepth": 0}))
			.await?;
		Ok(())
	}

	/// Connect to the BiDi websocket and spawn its socket pump.
	pub async fn connect(
		driver_url: &str,
		session_id: &str,
		socket_url: &str,
	) -> Result<Self> {
		let (cmd_tx, cmd_rx) = async_channel::unbounded::<String>();

		let inner = Arc::new(SessionInner {
			driver_url: driver_url.to_string(),
			session_id: session_id.to_string(),
			socket_url: socket_url.to_string(),
			next_id: AtomicUsize::new(1),
			pending: Mutex::new(HashMap::new()),
			cmd_tx,
			subscribers: Mutex::new(Vec::new()),
		});

		let ready = Self::spawn_pump(inner.clone(), cmd_rx);
		ready
			.recv()
			.await
			.map_err(|_| bevyhow!("socket pump exited before connecting"))??;

		Ok(Self { inner })
	}

	/// Spawn the socket pump: a single task that connects the socket, drains
	/// `cmd_rx` into outbound text frames, and routes inbound frames to the
	/// pending map (responses) or the events channel.
	///
	/// The socket's halves are thread-bound (`SendWrapper`), so the whole
	/// lifecycle lives in one `spawn_local` task: created and polled on the same
	/// thread, never migrated across the pool. The returned channel yields the
	/// connect result once the socket is up.
	fn spawn_pump(
		inner: Arc<SessionInner>,
		cmd_rx: async_channel::Receiver<String>,
	) -> async_channel::Receiver<Result<()>> {
		let (ready_tx, ready_rx) = async_channel::bounded::<Result<()>>(1);
		IoTaskPool::get()
			.spawn_local(async move {
				let socket = match Socket::connect(&inner.socket_url).await {
					Ok(socket) => {
						ready_tx.send(Ok(())).await.ok();
						socket
					}
					Err(err) => {
						ready_tx.send(Err(err)).await.ok();
						return;
					}
				};
				let (mut send, mut recv) = socket.split();

				// outbound: drain the command channel; when it closes (`kill`) or a
				// send fails, close the socket gracefully on this thread.
				let writer = async move {
					while let Ok(raw) = cmd_rx.recv().await {
						if send.send(Message::text(raw)).await.is_err() {
							break;
						}
					}
					// Ignore close errors – session already deleted or socket gone.
					let _ = send.close(None).await;
				};

				// inbound: route responses to their pending sender, events best-effort.
				let reader = async move {
					while let Some(item) = recv.next().await {
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

						// Event (has method, no id): deliver to matching
						// subscribers, pruning any whose receiver has closed.
						if let Some(method) =
							val.get("method").and_then(|m| m.as_str())
						{
							let method = method.to_string();
							let mut subscribers =
								inner.subscribers.lock().unwrap();
							subscribers.retain(|(prefix, tx)| {
								if !method.starts_with(prefix.as_str()) {
									return !tx.is_closed();
								}
								// unbounded, so try_send only fails when closed
								tx.try_send(val.clone()).is_ok()
							});
						}
					}
				};

				// either half ending (channel closed, socket dropped) ends the pump;
				// both halves drop here, on the thread that created them.
				beet_core::exports::futures_lite::future::or(writer, reader)
					.await;
			})
			.detach();
		ready_rx
	}
}

#[cfg(test)]
mod test {
	use crate::webdriver::*;
	use beet_core::prelude::*;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn works() {
		let client =
			ClientProcess::new_with_opts(test_fixtures::client())
				.unwrap();
		let session = client.new_session().await.unwrap();
		// Simple BiDi round‑trip health check.
		session.ping().await.unwrap();
		session.kill().await.unwrap();
		client.kill().unwrap();
	}
}

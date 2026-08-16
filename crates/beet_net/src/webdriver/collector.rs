//! Typed event collectors over the session's BiDi event stream.
//!
//! A collector pairs a [`Session::events`] receiver with a parse function,
//! accumulating typed events for later assertion ([`Collector::drain`]) or
//! live streaming ([`Collector::recv`]). Construct via [`Page::console`] and
//! [`Page::network`], which also issue the `session.subscribe`.

use super::Page;
use beet_core::exports::async_channel;
use beet_core::prelude::*;
use serde_json::Value;

/// A typed accumulator over one BiDi event subscription.
pub struct Collector<T> {
	rx: async_channel::Receiver<Value>,
	parse: fn(&Value) -> Option<T>,
}

impl<T> Collector<T> {
	pub(crate) fn new(
		rx: async_channel::Receiver<Value>,
		parse: fn(&Value) -> Option<T>,
	) -> Self {
		Self { rx, parse }
	}

	/// All events received so far, non-blocking.
	pub fn drain(&self) -> Vec<T> {
		let mut items = Vec::new();
		while let Ok(event) = self.rx.try_recv() {
			if let Some(item) = (self.parse)(&event) {
				items.push(item);
			}
		}
		items
	}

	/// Await the next event, skipping frames the parser rejects.
	pub async fn recv(&self) -> Result<T> {
		loop {
			let event = self
				.rx
				.recv()
				.await
				.map_err(|_| bevyhow!("event stream closed"))?;
			if let Some(item) = (self.parse)(&event) {
				return Ok(item);
			}
		}
	}
}

/// One console line (or uncaught error) from `log.entryAdded`.
#[derive(Debug, Clone)]
pub struct ConsoleEntry {
	/// `debug` | `info` | `warn` | `error`.
	pub level: SmolStr,
	/// The rendered message text.
	pub text: String,
}

impl ConsoleEntry {
	/// Whether this entry is an error, including uncaught exceptions.
	pub fn is_error(&self) -> bool { self.level == "error" }

	fn parse(event: &Value) -> Option<Self> {
		let params = event.get("params")?;
		Some(Self {
			level: params.get("level")?.as_str()?.into(),
			text: params
				.get("text")
				.and_then(|text| text.as_str())
				.unwrap_or_default()
				.to_string(),
		})
	}
}

/// One outgoing request from `network.beforeRequestSent`.
#[derive(Debug, Clone)]
pub struct RequestEvent {
	/// The request method, eg `GET`.
	pub method: SmolStr,
	/// The full request url.
	pub url: String,
}

impl RequestEvent {
	fn parse(event: &Value) -> Option<Self> {
		let request = event.pointer("/params/request")?;
		Some(Self {
			method: request.get("method")?.as_str()?.into(),
			url: request.get("url")?.as_str()?.to_string(),
		})
	}
}

/// One completed response from `network.responseCompleted`.
#[derive(Debug, Clone)]
pub struct ResponseEvent {
	/// The http status code.
	pub status: u16,
	/// The full request url.
	pub url: String,
}

impl ResponseEvent {
	/// Whether this response failed (4xx/5xx). A failed subresource (eg a
	/// 403 favicon) raises no console error, so a client check needs both.
	pub fn is_error(&self) -> bool { self.status >= 400 }

	fn parse(event: &Value) -> Option<Self> {
		let response = event.pointer("/params/response")?;
		Some(Self {
			status: response.get("status")?.as_u64()? as u16,
			url: response.get("url")?.as_str()?.to_string(),
		})
	}
}

impl Page {
	/// Collect console output (and uncaught errors) for this page. The
	/// receiver is registered before the subscribe resolves, so nothing
	/// logged after the call can slip past.
	pub async fn console(&self) -> Result<Collector<ConsoleEntry>> {
		let rx = self.session.events("log.entryAdded");
		self.session
			.subscribe(&["log.entryAdded"], Some(&[&self.context_id]))
			.await?;
		Collector::new(rx, ConsoleEntry::parse).xok()
	}

	/// Collect every request this page sends, from the moment of the call.
	pub async fn network(&self) -> Result<Collector<RequestEvent>> {
		let rx = self.session.events("network.beforeRequestSent");
		self.session
			.subscribe(
				&["network.beforeRequestSent"],
				Some(&[&self.context_id]),
			)
			.await?;
		Collector::new(rx, RequestEvent::parse).xok()
	}

	/// Collect every completed response, from the moment of the call.
	pub async fn responses(&self) -> Result<Collector<ResponseEvent>> {
		let rx = self.session.events("network.responseCompleted");
		self.session
			.subscribe(
				&["network.responseCompleted"],
				Some(&[&self.context_id]),
			)
			.await?;
		Collector::new(rx, ResponseEvent::parse).xok()
	}
}

#[cfg(test)]
mod test {
	use crate::webdriver::test_fixtures;
	use beet_core::prelude::*;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn collects_console_and_network() {
		App::default()
			.run_io_task_local(async move {
				let url = test_fixtures::page_url(
					"collector",
					r#"<html><body>
					<button id="log">log</button>
					<script>
					document.querySelector('#log').addEventListener('click', () => {
						console.log('clicked!');
						console.error('oh no');
					});
					</script>
					</body></html>"#,
				);
				let (proc, page) = test_fixtures::visit(&url).await;
				let console = page.console().await.unwrap();
				let network = page.network().await.unwrap();

				page.find("#log").await.unwrap().click().await.unwrap();

				let mut entries = Vec::new();
				poll_ext::poll_async(async || {
					entries.extend(console.drain());
					(entries.len() >= 2).then_some(()).ok_or_else(|| {
						bevyhow!("only {} entries", entries.len())
					})
				})
				.await
				.unwrap();
				entries
					.iter()
					.any(|entry| {
						entry.text.contains("clicked!") && !entry.is_error()
					})
					.xpect_true();
				entries
					.iter()
					.any(|entry| entry.text.contains("oh no") && entry.is_error())
					.xpect_true();

				// a local click round-trip sends nothing over the network
				network.drain().xpect_empty();

				page.kill().await.unwrap();
				proc.kill().unwrap();
			})
			.await;
	}
}

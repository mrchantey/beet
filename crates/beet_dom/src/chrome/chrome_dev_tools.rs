use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use tokio::process::Child;
use tokio::process::Command;
use tokio::time::Duration;
use tokio_tungstenite::connect_async;

pub fn net_backoff() -> Backoff {
	let attempts = 10;
	let min = Duration::from_millis(10);
	let max = Duration::from_secs(10);
	Backoff::new(attempts, min, max)
}


/// Retries a function with sensible defaults for network requests,
/// 10 requests ranging from 10ms to 10 seconds
pub async fn retry_net<Fut, O>(mut func: impl FnMut() -> Fut) -> Result<O>
where
	Fut: Future<Output = Result<O>>,
{
	for duration in net_backoff() {
		match (func().await, duration) {
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
	unreachable!();
}



pub struct ChromeDevTools {
	port: u16,
	process: Child,
}

impl ChromeDevTools {
	pub async fn connect() -> Result<Self> {
		Self::connect_with_port(9222).await
	}
	pub fn port(&self) -> u16 { self.port }

	pub async fn connect_with_port(port: u16) -> Result<Self> {
		let child = Command::new("google-chrome")
			.arg("--headless")
			.arg(format!("--remote-debugging-port={port}"))
			.arg("--disable-gpu")
			.kill_on_drop(true)
			.spawn()?;

		Self {
			port,
			process: child,
		}
		.xok()
	}

	/// after spawned, get the socket url using exponential backoff
	async fn socket_url(&self, conn_type: &str) -> Result<String> {
		let res = retry_net(async || {
			Request::get(&format!("http://127.0.0.1:{}/json", self.port))
				.send()
				.await
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

	pub async fn visit(&self, url: impl AsRef<str>) -> Result<Page> {
		let socket_url = self.socket_url("page").await?;
		let (ws, _) = connect_async(socket_url).await?;
		let mut page = Page::new(ws);
		page.visit(url).await?;
		Ok(page)
	}

	pub async fn kill(mut self) -> Result<()> {
		self.process.kill().await?;
		Ok(())
	}
}

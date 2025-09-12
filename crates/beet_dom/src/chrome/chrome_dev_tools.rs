use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use tokio::process::Child;
use tokio::process::Command;

pub(super) const DEFAULT_PORT: u16 = 9222;

pub struct ChromeDevTools {
	port: u16,
	process: Child,
}

impl ChromeDevTools {
	pub async fn spawn() -> Result<Self> {
		Self::spawn_with_port(DEFAULT_PORT).await
	}

	pub fn port(&self) -> u16 { self.port }

	pub async fn spawn_with_port(port: u16) -> Result<Self> {
		let child = Command::new("nix-shell")
			.arg("-p")
			.arg("chromium")
			.arg("--run")
			.arg(format!("'bash -lc \"chromium --headless=new --disable-gpu --remote-debugging-port={port} --auto-open-devtools-for-tabs --user-data-dir=$(mktemp -d)\"'"))
			.kill_on_drop(true)
			.spawn()?;
		// --ozone-platform=wayland --enable-features=UseOzonePlatform
		Self {
			port,
			process: child,
		}
		.xok()
	}

	/// Get the socket url using exponential backoff
	pub async fn socket_url(url: &Url, conn_type: &str) -> Result<String> {
		let res = Backoff::default()
			.retry_async(async |_| Request::get(&url).send().await)
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

	pub async fn kill(mut self) -> Result<()> {
		self.process.kill().await?;
		Ok(())
	}
}

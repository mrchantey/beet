use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use serde_json::json;
use std::borrow::Cow;
use std::time::Duration;
#[cfg(feature = "tokio")]
use tokio::process::Child;
#[cfg(feature = "tokio")]
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Client {
	host: Cow<'static, str>,
	provider: Provider,
	driver_port: u16,
	/// Port to serve bidi websockets on.
	/// this is for geckodriver only, chromedriver uses
	/// the same `driver_port`.
	websocket_port: u16,
	log_level: LogLevel,
}

#[derive(Debug, Default, Clone)]
pub enum LogLevel {
	#[default]
	Info,
	Debug,
	Warn,
	Error,
	Off,
}

impl Default for Client {
	fn default() -> Self {
		Self {
			driver_port: DEFAULT_WEBDRIVER_PORT,
			websocket_port: DEFAULT_WEBDRIVER_SESSION_PORT,
			host: "http://127.0.0.1".into(),
			provider: default(),
			log_level: LogLevel::Warn,
		}
	}
}

/// Specify the browser driver process to use, defaults to chromedriver
#[derive(Debug, Default, Clone)]
pub enum Provider {
	#[default]
	Chromedriver,
	Geckodriver,
}

pub struct NewSessionOptions {
	headless: bool,
	disable_gpu: bool,
}

impl Default for NewSessionOptions {
	fn default() -> Self {
		Self {
			headless: true,
			disable_gpu: true,
		}
	}
}

impl Client {
	pub fn chromium() -> Self {
		Self {
			provider: Provider::Chromedriver,
			..default()
		}
	}
	pub fn firefox() -> Self {
		Self {
			provider: Provider::Geckodriver,
			..default()
		}
	}

	pub fn base_url(&self) -> String {
		format!("{}:{}", self.host, self.driver_port)
	}
	pub fn url(&self, path: &str) -> String {
		format!("{}:{}/{}", self.host, self.driver_port, path)
	}
	pub async fn new_session(&self) -> Result<Session> {
		Self::new_session_with_opts(self, default()).await
	}

	/// Create a new session, using an exponential backoff to await
	/// driver process creation if needed.
	pub async fn new_session_with_opts(
		&self,
		opts: NewSessionOptions,
	) -> Result<Session> {
		let browser_name = match self.provider {
			Provider::Chromedriver => "chrome",
			Provider::Geckodriver => "firefox",
		};

		let mut body = json!({
			"capabilities": {
				"alwaysMatch": {
					"browserName": browser_name,
					"webSocketUrl": true
				}
			}
		});

		match self.provider {
			Provider::Chromedriver => {
				let mut args = vec![
					// remote-debugging-port results in 'cannot connect to renderer'
					"--remote-debugging-pipe".into(),
					// "--disable-dev-shm-usage".into(),
					// "--no-sandbox".into(),
					// "--disable-software-rasterizer".into(),
				];
				if opts.headless {
					args.push("--headless=new".to_string());
				}
				if opts.disable_gpu {
					args.push("--disable-gpu".to_string());
				}
				body["capabilities"]["alwaysMatch"]
					.set_field("goog:chromeOptions", json!({ "args": args }))?;
			}
			Provider::Geckodriver => {
				let mut args = vec![];
				if opts.headless {
					// geckodriver expects "-headless"
					args.push("-headless".to_string());
				}

				body["capabilities"]["alwaysMatch"]
					.set_field("moz:firefoxOptions", json!({ "args": args }))?;
			}
		};

		let res = Backoff::default()
			.with_max_attempts(15)
			.with_max(Duration::from_secs(1))
			.retry_async(async |_| {
				Request::post(self.url("session"))
					.with_json_body(&body)?
					.send()
					.await?
					.into_result()
					.await?
					.json::<Value>()
					.await
			})
			.await?;

		// let value = res.field("value")?;/
		let session_id = res["value"]["sessionId"].to_str()?;
		let socket_url =
			res["value"]["capabilities"]["webSocketUrl"].to_str()?;

		let driver_url = self.base_url();
		Session::connect(&driver_url, session_id, socket_url).await
	}
}


#[cfg(feature = "tokio")]
pub struct ClientProcess {
	client: Client,
	process: Child,
}

impl std::ops::Deref for ClientProcess {
	type Target = Client;
	fn deref(&self) -> &Self::Target { &self.client }
}

#[cfg(feature = "tokio")]
impl ClientProcess {
	pub fn new() -> Result<Self> { Self::new_with_opts(default()) }
	pub fn new_with_opts(opts: Client) -> Result<Self> {
		let process = match opts.provider {
			Provider::Chromedriver => Self::spawn_chromedriver(&opts),
			Provider::Geckodriver => Self::spawn_geckodriver(&opts),
		}?;
		Self {
			client: opts,
			process,
		}
		.xok()
	}

	/// start the chromedriver and return the child process
	fn spawn_chromedriver(opts: &Client) -> Result<Child> {
		let run = vec![
			"chromedriver".into(),
			format!("--port={}", opts.driver_port),
			format!("--log-level={}", match opts.log_level {
				LogLevel::Info => "INFO",
				LogLevel::Debug => "DEBUG",
				LogLevel::Warn => "WARNING",
				LogLevel::Error => "SEVERE",
				LogLevel::Off => "OFF",
			}),
		];
		Command::new("nix-shell")
			.args(&["-p", "chromium", "chromedriver", "--run", &run.join(" ")])
			.kill_on_drop(true)
			.spawn()?
			.xok()
	}
	/// start geckodriver and return the child process
	fn spawn_geckodriver(opts: &Client) -> Result<Child> {
		let run = vec![
			"geckodriver".into(),
			format!("--port={}", opts.driver_port),
			format!("--websocket-port={}", opts.websocket_port),
			format!("--log={}", match opts.log_level {
				LogLevel::Info => "info",
				LogLevel::Debug => "debug",
				LogLevel::Warn => "warn",
				LogLevel::Error => "error",
				LogLevel::Off => "fatal",
			}),
		];

		Command::new("nix-shell")
			.args(&["-p", "firefox", "geckodriver", "--run", &run.join(" ")])
			.kill_on_drop(true)
			.spawn()?
			.xok()
	}

	pub async fn kill(mut self) -> Result<()> {
		self.process.kill().await?;
		Ok(())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[sweet::test]
	#[ignore = "smoketest"]
	async fn firefox() {
		let client = Client::firefox();
		let client = ClientProcess::new_with_opts(client.clone()).unwrap();
		let session = client.new_session().await.unwrap();
		session.kill().await.unwrap();
		client.kill().await.unwrap();
	}
	#[sweet::test]
	// #[ignore = "smoketest"]
	async fn chromium() {
		let client = ClientProcess::new_with_opts(Client {
			provider: Provider::Chromedriver,
			..default()
		})
		.unwrap();
		let session = client.new_session().await.unwrap();
		session.kill().await.unwrap();
		client.kill().await.unwrap();
	}
}

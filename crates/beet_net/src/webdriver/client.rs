//! WebDriver client for browser automation.
//!
//! This module provides the [`Client`] type for connecting to WebDriver
//! instances and creating browser sessions.

use super::*;
use crate::prelude::*;
use async_process::Child;
use async_process::Command;
use async_process::Stdio;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;
use std::borrow::Cow;
use std::time::Duration;

/// WebDriver client configuration for browser automation.
#[derive(Debug, Clone, Get, SetWith)]
pub struct Client {
	/// The driver host, default `http://127.0.0.1`.
	host: Cow<'static, str>,
	/// The browser driver process to speak to.
	provider: Provider,
	/// Port the driver process listens on. Concurrent drivers (eg parallel
	/// tests) each need their own.
	driver_port: u16,
	/// Port to serve bidi websockets on.
	/// this is for geckodriver only, chromedriver uses
	/// the same `driver_port`.
	websocket_port: u16,
	/// Log verbosity for the driver process.
	log_level: LogLevel,
}

/// Log level configuration for the WebDriver process.
#[derive(Debug, Default, Clone)]
pub enum LogLevel {
	/// Info level logging.
	#[default]
	Info,
	/// Debug level logging.
	Debug,
	/// Warning level logging.
	Warn,
	/// Error level logging.
	Error,
	/// Disable logging.
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

/// Specifies the browser driver process to use.
#[derive(Debug, Default, Clone, Copy)]
pub enum Provider {
	/// Chrome/Chromium via chromedriver.
	#[default]
	Chromedriver,
	/// Firefox via geckodriver.
	Geckodriver,
}

/// Options for creating a new WebDriver session.
#[derive(Debug, Clone, Get, SetWith)]
pub struct NewSessionOptions {
	/// Whether to run the browser in headless mode, default `true`. Turn off
	/// to watch a test drive the browser locally.
	headless: bool,
	/// Whether to disable GPU acceleration, default `true`.
	disable_gpu: bool,
	/// Extra flags appended to the browser launch args verbatim, eg
	/// `--enable-unsafe-webgpu` (pair with `disable_gpu: false`) for a
	/// WebGPU-granting headless chrome.
	extra_args: Vec<String>,
}

impl Default for NewSessionOptions {
	fn default() -> Self {
		Self {
			headless: true,
			disable_gpu: true,
			extra_args: Vec::new(),
		}
	}
}

impl Client {
	/// A client with a process-unique driver/websocket port pair, so
	/// concurrently running tests and harnesses never fight over one driver
	/// process. Pairs advance from a per-process base above
	/// [`DEFAULT_WEBDRIVER_PORT`], spread by process id so concurrently running
	/// test *binaries* rarely collide either.
	pub fn unique() -> Self {
		use std::sync::atomic::AtomicU16;
		use std::sync::atomic::Ordering;
		static NEXT_PAIR: AtomicU16 = AtomicU16::new(0);
		let pair = NEXT_PAIR.fetch_add(1, Ordering::SeqCst);
		let base = DEFAULT_WEBDRIVER_PORT
			+ 10 + (std::process::id() as u16 % 512) * 32
			+ pair * 2;
		Self::default()
			.with_driver_port(base)
			.with_websocket_port(base + 1)
	}

	/// Creates a client configured for Chromium.
	pub fn chromium() -> Self {
		Self {
			provider: Provider::Chromedriver,
			..default()
		}
	}
	/// Creates a client configured for Firefox.
	pub fn firefox() -> Self {
		Self {
			provider: Provider::Geckodriver,
			..default()
		}
	}

	/// Returns the base URL for the WebDriver server.
	pub fn base_url(&self) -> String {
		format!("{}:{}", self.host, self.driver_port)
	}
	/// Returns a full URL for the given path on the WebDriver server.
	pub fn url(&self, path: &str) -> String {
		format!("{}:{}/{}", self.host, self.driver_port, path)
	}
	/// Creates a new browser session with default options.
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
				args.extend(opts.extra_args.iter().cloned());
				body["capabilities"]["alwaysMatch"]
					.set_field("goog:chromeOptions", json!({ "args": args }))?;
			}
			Provider::Geckodriver => {
				let mut args = vec![];
				if opts.headless {
					// geckodriver expects "-headless"
					args.push("-headless".to_string());
				}
				args.extend(opts.extra_args.iter().cloned());
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

/// A WebDriver client with an owned driver process.
///
/// This struct manages the lifecycle of the WebDriver process,
/// killing it when dropped.
#[derive(Get)]
pub struct ClientProcess {
	/// The underlying webdriver client
	client: Client,
	/// The chromedriver or geckodriver process
	/// driving the webdriver interaction
	process: Child,
}

impl std::ops::Deref for ClientProcess {
	type Target = Client;
	fn deref(&self) -> &Self::Target { &self.client }
}

impl ClientProcess {
	/// Creates a new client process with default options.
	pub fn new() -> Result<Self> { Self::new_with_opts(default()) }

	/// Creates a new client process with the given options.
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

	/// Verify that the driver and a browser binary for `provider` are
	/// discoverable on `PATH`. The browser ships under different names per
	/// distro (github runners have `google-chrome`, arch has `chromium`), so
	/// any candidate satisfies the check. Returns an error containing
	/// platform-specific install instructions on a miss.
	pub async fn check_installed(provider: Provider) -> Result<()> {
		let (driver, browsers): (_, &[_]) = match provider {
			Provider::Chromedriver => ("chromedriver", &[
				"chromium",
				"chromium-browser",
				"google-chrome",
				"google-chrome-stable",
				"chrome",
			]),
			Provider::Geckodriver => {
				("geckodriver", &["firefox", "firefox-esr"])
			}
		};
		let mut missing = Vec::new();
		if !Self::binary_available(driver).await {
			missing.push(driver.to_string());
		}
		let mut browser_found = false;
		for browser in browsers {
			if Self::binary_available(browser).await {
				browser_found = true;
				break;
			}
		}
		if !browser_found {
			missing.push(browsers.join("|"));
		}
		if missing.is_empty() {
			return Ok(());
		}
		bevybail!(
			"webdriver dependencies missing: {missing}\n{instructions}",
			missing = missing.join(", "),
			instructions = Self::install_instructions(provider)
		);
	}

	/// Check whether `binary` can be invoked with `--version`.
	async fn binary_available(binary: &str) -> bool {
		Command::new(binary)
			.arg("--version")
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.await
			.map(|status| status.success())
			.unwrap_or(false)
	}

	/// Per-platform install hints for the given provider.
	fn install_instructions(provider: Provider) -> String {
		let (linux_arch, linux_debian, macos, windows_url) = match provider {
			Provider::Chromedriver => (
				"sudo pacman -S chromium",
				"sudo apt install chromium-browser chromium-chromedriver",
				"brew install --cask chromium && brew install chromedriver",
				"https://googlechromelabs.github.io/chrome-for-testing/",
			),
			Provider::Geckodriver => (
				"sudo pacman -S firefox geckodriver",
				"sudo apt install firefox-esr && cargo install geckodriver",
				"brew install --cask firefox && brew install geckodriver",
				"https://github.com/mozilla/geckodriver/releases",
			),
		};
		format!(
			"install with:\n  linux (arch):   {linux_arch}\n  \
			 linux (debian): {linux_debian}\n  \
			 macos:          {macos}\n  \
			 windows:        see {windows_url}"
		)
	}

	/// start the chromedriver and return the child process
	fn spawn_chromedriver(opts: &Client) -> Result<Child> {
		Command::new("chromedriver")
			.arg(format!("--port={}", opts.driver_port))
			.arg(format!("--log-level={}", match opts.log_level {
				LogLevel::Info => "INFO",
				LogLevel::Debug => "DEBUG",
				LogLevel::Warn => "WARNING",
				LogLevel::Error => "SEVERE",
				LogLevel::Off => "OFF",
			}))
			.kill_on_drop(true)
			.spawn()?
			.xok()
	}
	/// start geckodriver and return the child process
	fn spawn_geckodriver(opts: &Client) -> Result<Child> {
		Command::new("geckodriver")
			.arg(format!("--port={}", opts.driver_port))
			.arg(format!("--websocket-port={}", opts.websocket_port))
			.arg(format!("--log={}", match opts.log_level {
				LogLevel::Info => "info",
				LogLevel::Debug => "debug",
				LogLevel::Warn => "warn",
				LogLevel::Error => "error",
				LogLevel::Off => "fatal",
			}))
			.kill_on_drop(true)
			.spawn()?
			.xok()
	}

	/// Kills the WebDriver process.
	pub fn kill(mut self) -> Result<()> {
		self.process.kill()?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::webdriver::test_fixtures;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn firefox() {
		// dev machines often lack geckodriver; ci pins its presence with an
		// explicit verify step, so a graceful skip here masks nothing there
		if ClientProcess::check_installed(Provider::Geckodriver)
			.await
			.is_err()
		{
			warn!("geckodriver/firefox not on PATH, skipping");
			return;
		}
		let client = ClientProcess::new_with_opts(
			test_fixtures::client().with_provider(Provider::Geckodriver),
		)
		.unwrap();
		let session = client.new_session().await.unwrap();
		session.kill().await.unwrap();
		client.kill().unwrap();
	}
	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn chromium() {
		let client =
			ClientProcess::new_with_opts(test_fixtures::client()).unwrap();
		let session = client.new_session().await.unwrap();
		session.kill().await.unwrap();
		client.kill().unwrap();
	}
}

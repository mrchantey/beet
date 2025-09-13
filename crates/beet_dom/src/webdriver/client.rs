use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use fantoccini::ClientBuilder;
use serde_json::json;
use tokio::process::Child;
use tokio::process::Command;

pub struct WebdriverClient {
	client: fantoccini::Client,
	/// The chromedriver process, if specified to spawn
	/// in [`ConnectOptions`]
	chromedriver: Option<Child>,
}

impl WebdriverClient {
	pub async fn connect() -> Result<Self> {
		Self::connect_with_opts(default()).await
	}
	pub fn page(&self) -> Page {
		let provider = WebdriverPage::new(self.client.clone());
		Page::new(provider)
	}


	pub async fn connect_with_opts(opts: ConnectOptions) -> Result<Self> {
		let chromedriver = if opts.spawn_process {
			Some(Self::spawn_chromedriver(opts.port)?)
		} else {
			None
		};

		let args = if opts.headless {
			vec!["--headless", "--disable-gpu"]
		} else {
			vec![]
		};
		let capabilities = json! ({
			"browserName" : "chrome",
			"goog:chromeOptions":{
				"args":args
			}
		});
		let capabilities = capabilities.to_object().unwrap();

		let url = opts.url();
		let capabilities = capabilities;
		let client = Backoff::default()
			.with_max_attempts(20)
			.retry_async(async |_| {
				ClientBuilder::native()
					.capabilities(capabilities.clone())
					.connect(&url)
					.await
			})
			.await?;

		Self {
			client,
			chromedriver,
		}
		.xok()
	}
	/// start the chromedriver and return the child process
	/// Note: Unlike chrome dev tools this starts the *chromedriver* process,
	/// not the actual browser. The browser will be spawned by
	fn spawn_chromedriver(port: u16) -> Result<Child> {
		/*
		Options
		--port=PORT                     port to listen on
		--adb-port=PORT                 adb server port
		--log-path=FILE                 write server log to file instead of stderr, increases log level to INFO
		--log-level=LEVEL               set log level: ALL, DEBUG, INFO, WARNING, SEVERE, OFF
		--verbose                       log verbosely (equivalent to --log-level=ALL)
		--silent                        log nothing (equivalent to --log-level=OFF)
		--append-log                    append log file instead of rewriting
		--replayable                    (experimental) log verbosely and don't truncate long strings so that the log can be replayed.
		--version                       print the version number and exit
		--url-base                      base URL path prefix for commands, e.g. wd/url
		--readable-timestamp            add readable timestamps to log
		--enable-chrome-logs            show logs from the browser (overrides other logging options)
		--bidi-mapper-path              custom bidi mapper path
		--disable-dev-shm-usage         do not use /dev/shm (add this switch if seeing errors related to shared memory)
		--ignore-explicit-port          (experimental) ignore the port specified explicitly, find a free port instead
		--allowed-ips=LIST              comma-separated allowlist of remote IP addresses which are allowed to connect to ChromeDriver
		--allowed-origins=LIST          comma-separated allowlist of request origins which are allowed to connect to ChromeDriver. Using `*` to allow any host origin is dangerous!
		*/

		Command::new("nix-shell")
			.args(&[
				"-p",
				"chromium",
				"chromedriver",
				"--run",
				&format!("chromedriver --port={port} --silent"),
			])
			.spawn()?
			.xok()
	}

	pub async fn kill(self) -> Result<()> {
		self.client.close().await?;
		if let Some(mut child) = self.chromedriver {
			child.kill().await?;
		}
		Ok(())
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	async fn works() {
		let client = WebdriverClient::connect().await.unwrap();
		client.kill().await.unwrap();
	}
}

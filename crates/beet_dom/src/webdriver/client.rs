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

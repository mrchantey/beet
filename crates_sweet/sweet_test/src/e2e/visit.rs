use crate::prelude::*;
use anyhow::Result;
use fantoccini::Client;
use fantoccini::ClientBuilder;
use std::time::Duration;
use sweet_utils::prelude::*;
use sweet_utils::utils::AsyncUtils;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunTestsMode {
	Headless,
	Headed,
}

pub struct VisitOptions {
	/// Sometimes webdriver takes a moment to start up,
	/// we will retry until it is available
	pub timeout: Duration,
	pub headless: bool,
	pub webdriver_port: u16,
}
impl Default for VisitOptions {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(5),
			headless: true,
			webdriver_port: 4444,
		}
	}
}

/// Visit a page, returning the [Page]
/// ## Panics
/// - If the webdriver is not running
/// - If the page cannot be reached
pub async fn visit(url: &str) -> Page {
	match visit_with_opts(url, VisitOptions::default()).await {
		Ok(page) => page,
		Err(err) => {
			eprintln!(
				"Error visiting page: {}\n\nPlease ensure the --e2e flag was passed to the test: \n\n`cargo test --lib -- --e2e`\n\n",
				err
			);
			std::process::exit(1);
		}
	}
}


/// Visit a page, returning the [Page]
/// ## Panics
/// - If the webdriver is not running
/// - If the page cannot be reached
pub async fn visit_with_opts(url: &str, opts: VisitOptions) -> Result<Page> {
	let client = AsyncUtils::retry_async(
		async || -> Result<Client> {
			let headless_args = if opts.headless {
				r#""--headless","--disable-gpu""#
			} else {
				""
			};
			let cap = serde_json::from_str(&format!(
				r#"{{
						"browserName" : "chrome",
						"goog:chromeOptions": {{
							"args": [{headless_args}]
					}}
				}}"#
			))?;
			let client = ClientBuilder::native()
				.capabilities(cap)
				.connect(&format!("http://localhost:{}", opts.webdriver_port))
				.await?;
			Ok(client)
		},
		opts.timeout,
		Duration::from_millis(100),
	)
	.await?;
	client.goto(url).await?;
	Page::new(client).xok()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use fantoccini::Locator;

	#[crate::test]
	async fn works() -> Result<()> {
		let c = visit("https://en.wikipedia.org/wiki/Foobar").await.client;
		let url = c.current_url().await?;
		assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
		c.find(Locator::Css(".mw-disambig")).await?.click().await?;
		c.find(Locator::LinkText("Foo Lake")).await?.click().await?;
		let url = c.current_url().await?;
		assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
		Ok(())
	}
}

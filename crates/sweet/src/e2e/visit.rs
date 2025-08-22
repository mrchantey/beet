use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::*;
use fantoccini::Client;
use fantoccini::ClientBuilder;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunTestsMode {
	Headless,
	Headed,
}

pub const DEFAULT_WEBDRIVER_PORT: u16 = 4444;

pub struct VisitOptions {
	/// Sometimes webdriver takes a moment to start up,
	/// we will retry until it is available
	pub timeout: Duration,
	pub headless: bool,
	/// The port for the webdriver process
	pub webdriver_port: u16,
}
impl Default for VisitOptions {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(5),
			headless: true,
			webdriver_port: DEFAULT_WEBDRIVER_PORT,
		}
	}
}

/// Serves the axum router on a port incremented from {DEFAULT_WEBDRIVER_PORT}
/// so test routers can be served concurrently. The port is then used
/// to prepend the provided path, so `/foo` becomes `http://127.0.0.1:4445/foo`
pub async fn serve_and_visit(
	router: axum::Router,
	path: impl AsRef<std::path::Path>,
) -> (Page, (tokio::task::JoinHandle<()>, u16)) {
	use std::sync::atomic::AtomicU16;
	use std::sync::atomic::Ordering;

	static NEXT_PORT: AtomicU16 = AtomicU16::new(DEFAULT_WEBDRIVER_PORT + 1);
	let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
	let addr = format!("127.0.0.1:{}", port);
	let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});
	let url = format!("http://{addr}{}", path.as_ref().display());
	let page = visit(&url).await;

	(page, (server, port))
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
				r#"
Error visiting page: {}
This is usually either an issue with webdriver or the the e2e flag not being set.
Please ensure the --e2e flag was passed to the test:
`cargo test --lib -- --e2e`
"#,
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
	let client = async_ext::retry(
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
	use crate::e2e::serve_and_visit;
	use crate::prelude::*;
	use anyhow::Result;
	use fantoccini::Locator;

	#[crate::test]
	#[ignore="external url"]
	async fn works() -> Result<()> {
		let page = visit("https://en.wikipedia.org/wiki/Foobar").await.client;
		let url = page.current_url().await?;
		assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
		page.find(Locator::Css(".mw-disambig"))
			.await?
			.click()
			.await?;
		page.find(Locator::LinkText("Foo Lake"))
			.await?
			.click()
			.await?;
		let url = page.current_url().await?;
		expect(url.as_str()).to_be("https://en.wikipedia.org/wiki/Foo_Lake");
		Ok(())
	}

	#[crate::test]
	async fn test_serve_and_visit() -> Result<()> {
		use axum::Router;
		use axum::routing::get;
		let router = Router::new().route("/foo", get(async || "hello world!"));
		let (page, _) = serve_and_visit(router, "/foo").await;
		let url = page.current_url().await?;
		expect(url).to_end_with("/foo");
		let body = page.find(Locator::Css("body")).await?.text().await?;
		expect(body).to_contain("hello world!");

		Ok(())
	}
}

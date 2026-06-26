//! WebDriver example
//!
//! This example spawns a chromedriver process, opens `example.com`,
//! reads the heading text and clicks the anchor to follow a link.
//!
//! Prerequisites: `chromedriver` and `chromium` must be available on `PATH`.
//!
//! Run with:
//! ```sh
//! cargo run --example webdriver --features webdriver
//! ```

use beet::net::prelude::webdriver::*;
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AsyncPlugin::default(),
		))
		.add_systems(Startup, run_webdriver)
		.run();
	info!("Done");
}

fn run_webdriver(async_commands: AsyncCommands) {
	async_commands.run_local(|world| async move {
		ClientProcess::check_installed(Provider::Chromedriver).await?;

		let (process, page) = Page::visit("https://example.com")
			.await
			.expect("failed to visit example.com");

		assert_eq!(
			page.current_url().await.expect("current_url failed"),
			"https://example.com/"
		);

		let heading = page
			.query_selector("h1")
			.await
			.expect("query failed")
			.expect("missing h1");
		assert_eq!(
			heading.inner_text().await.expect("inner_text failed"),
			"Example Domain"
		);

		let anchor = page
			.query_selector("a")
			.await
			.expect("query failed")
			.expect("missing anchor");
		anchor.click().await.expect("click failed");

		// wait for the navigation to land
		Backoff::default()
			.with_max_attempts(10)
			.retry_async(|_| async {
				match page.current_url().await.unwrap().as_str() {
					"https://www.iana.org/help/example-domains" => Ok(()),
					_ => Err(()),
				}
			})
			.await
			.expect("did not navigate to iana.org");

		page.kill().await.expect("session kill failed");
		process.kill().expect("driver kill failed");

		world.write_message(AppExit::Success).await;
		Ok(())
	});
}

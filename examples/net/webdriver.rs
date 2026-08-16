//! WebDriver example
//!
//! This example spawns a chromedriver process, opens `example.com`, reads the
//! heading text with an auto-waiting find, follows a link with a trusted
//! click, then saves a screenshot of the landing page.
//!
//! Prerequisites: `chromedriver` and a chromium/chrome must be on `PATH`.
//!
//! Run with:
//! ```sh
//! cargo run --example webdriver --features webdriver
//! ```

use beet::net::prelude::webdriver::*;
use beet::prelude::*;
// both preludes carry a `Provider` (the model streamer's is the other), so name
// the webdriver one explicitly: an explicit import beats a glob.
use beet::net::prelude::webdriver::Provider;

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

		let page = Browser::visit("https://example.com").await?;

		// try_find auto-waits for the selector, so no readiness dance
		let heading = page.try_find("h1").await?.inner_text().await?;
		info!("heading: {heading}");

		// the innerText locator + a trusted click follows the link
		page.try_find_text("More information...").await?.click().await?;
		poll_ext::poll_async(async || {
			let url = page.current_url().await?;
			url.contains("iana.org")
				.then_some(())
				.ok_or_else(|| bevyhow!("still at {url}"))
		})
		.await?;
		info!("landed on {}", page.current_url().await?);

		// capture the landing page
		let png = page.screenshot().await?;
		let out = fs_ext::workspace_root().join("target/webdriver-example.png");
		fs_ext::write(&out, &png)?;
		info!("saved screenshot to {}", out.display());

		page.kill().await?;

		world.write_message(AppExit::Success).await;
		Ok(())
	});
}

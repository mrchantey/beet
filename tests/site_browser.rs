//! The browser half of the deploy skill's client verification (check `b`),
//! the in-house replacement for the retired playwright `verify_client.js`:
//! drive the live site through the webdriver, fail on any client error, and
//! prove the counter's reactive round trip, in-page navigability, asset
//! store topology and mobile layout.
//!
//! Three collectors gate the whole run, attached before the first navigation
//! so nothing slips past: console errors (which include uncaught exceptions,
//! the `crypto.randomUUID` production-miss lesson), 4xx/5xx responses (a
//! failed favicon raises no console error), and the request log.
//!
//! Parameterized by `BEET_BASE_URL` so the same test verifies every stage:
//!
//! ```sh
//! cargo run -p beet-cli -- serve site --server=http   # note the bound port
//! BEET_BASE_URL=http://localhost:8337 cargo test --test site_browser \
//!   --features router,json,testing,webdriver -- --include-ignored
//! ```
beet::test_main!();

use beet::net::prelude::webdriver::*;
use beet::prelude::*;
use core::time::Duration;

#[beet_core::test(timeout_ms = 120_000)]
#[ignore = "smoketest"]
async fn verifies_client() {
	let Ok(base) = env_ext::var("BEET_BASE_URL") else {
		warn!("BEET_BASE_URL unset, skipping the site browser verification");
		return;
	};
	let base = base.trim_end_matches('/').to_string();
	// fixed ports above the per-test smoketest ranges
	let mut page = Browser::new_with(
		Client::default()
			.with_driver_port(8394)
			.with_websocket_port(8395),
	)
	.await
	.unwrap();
	let console = page.console().await.unwrap();
	let responses = page.responses().await.unwrap();
	// desktop viewport: the headless default (800x600) collapses the
	// sidebar into a drawer, hiding every nav link from hit-testing
	page.set_viewport(1280, 800).await.unwrap();

	// the counter round trip: trusted clicks patch the count text
	// locally (the reactive runtime, hydrated from the blob)
	page.navigate(&format!("{base}/docs/design/counter"))
		.await
		.unwrap();
	cross_log!("stage: counter page loaded");
	let more = page.find_text("More").await;
	more.click().await.unwrap();
	more.click().await.unwrap();
	wait_for_count(&page, "You have clicked 2 times").await;
	page.find_text("Less").await.click().await.unwrap();
	wait_for_count(&page, "You have clicked 1 times").await;

	// in-page navigability: the site's own links reach docs and the
	// counter (proves navigation, not just direct loads)
	cross_log!("stage: counter done");
	// `Page::click` re-queries on staleness: the client-side router
	// re-renders the page, invalidating held node references
	page.navigate(&base).await.unwrap();
	page.click("a[href='/docs']").await.unwrap();
	page.xpect_url(&format!("{base}/docs")).await;
	// the counter link on /docs sits in a collapsed nav subtree
	// (zero-size, unclickable for a real user too), so walk the
	// visible hierarchy: the design section first
	page.click("a[href='/docs/design']").await.unwrap();
	page.xpect_url(&format!("{base}/docs/design")).await;
	page.click("a[href='/docs/design/counter']").await.unwrap();
	page.xpect_url(&format!("{base}/docs/design/counter")).await;

	// the beacon runs on content pages, where the client error class
	// was originally reported
	cross_log!("stage: in-page nav done");
	for path in ["/blog", "/blog/post-3"] {
		page.navigate(&format!("{base}{path}")).await.unwrap();
		page.find("body").await;
	}

	// asset sweep on a page that carries an image: every `/assets/`
	// reference fetches 200 through the page itself (a private bucket
	// handing out public urls turns each into a redirect to a 403)
	cross_log!("stage: blog pages done");
	page.navigate(&format!("{base}/blog/post-6")).await.unwrap();
	let statuses = page
		.evaluate_value(
			r#"Promise.all(
				[...document.querySelectorAll('img[src],script[src],source[src],link[href]')]
					.map(el => el.src || el.href)
					.filter(url => url.includes('/assets/'))
					.map(async url => `${(await fetch(url)).status} ${url}`)
			)"#,
		)
		.await
		.unwrap();
	statuses
		.as_array()
		.unwrap()
		.iter()
		.filter(|entry| !entry.as_str().unwrap_or_default().starts_with("200 "))
		.collect::<Vec<_>>()
		.xpect_empty();

	// mobile layout: no horizontal overflow at phone widths (the matcher
	// prints the offending elements on failure), and the opened sidebar
	// drawer overlays the content rather than squeezing it into overflow
	cross_log!("stage: asset sweep done");
	for width in [375, 320] {
		page.set_viewport(width, 812).await.unwrap();
		for path in ["/", "/blog", "/blog/post-3"] {
			page.navigate(&format!("{base}{path}")).await.unwrap();
			page.find("body").await;
			page.xpect_no_horizontal_overflow().await;
			// open the drawer: still no overflow with the rail overlaid
			page.find("#menu-button").await.click().await.unwrap();
			page.find("#sidebar")
				.await
				.xpect_attr("aria-hidden", "false")
				.await;
			page.xpect_no_horizontal_overflow().await;
		}
	}

	// let straggler beacons and responses land before judging
	cross_log!("stage: viewports done");
	time_ext::sleep(Duration::from_millis(500)).await;
	console
		.drain()
		.into_iter()
		.filter(|entry| entry.is_error())
		.map(|entry| entry.text)
		.collect::<Vec<_>>()
		.xpect_empty();
	responses
		.drain()
		.into_iter()
		.filter(|response| response.is_error())
		.map(|response| format!("{} {}", response.status, response.url))
		.collect::<Vec<_>>()
		.xpect_empty();

	page.kill().await.unwrap();
}

/// Poll until the page body carries the expected count text.
async fn wait_for_count(page: &Page, expected: &str) {
	poll_ext::poll_async(async || {
		let text = page
			.evaluate_value("document.body.innerText")
			.await?
			.as_str()
			.unwrap_or_default()
			.to_string();
		text.contains(expected)
			.then_some(())
			.ok_or_else(|| bevyhow!("count not patched yet"))
	})
	.await
	.unwrap();
}

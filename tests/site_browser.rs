//! The browser half of the deploy skill's client verification (check `b`),
//! the in-house replacement for the retired playwright `verify_client.js`:
//! drive the live site through the webdriver, fail on any client error, and
//! prove the counter page's first paint, in-page navigability, that the
//! unlisted docs stay out of sight, asset store topology and the desktop and
//! mobile layouts.
//!
//! Client-side interactivity is not asserted here: the deployed site does not
//! yet boot the wasm runtime its pages could carry (`<Wasm/>`, see
//! `tests/bsx_site_browser.rs` for the example that does), so a served page is
//! first paint plus whatever bespoke page script it ships.
//!
//! Three collectors gate the whole run, attached before the first navigation
//! so nothing slips past: console errors (which include uncaught exceptions,
//! the `crypto.randomUUID` production-miss lesson), 4xx/5xx responses (a
//! failed favicon raises no console error), and the request log.
//!
//! Parameterized by `BEET_BASE_URL` so the same test verifies every stage:
//!
//! ```sh
//! cargo run -p beet-cli -- --entry=site serve --server=http   # note the bound port
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

	// the counter page's first paint: the `@doc:count=0` init reaches the
	// served html, and the buttons it declares are there to be found
	page.navigate(&format!("{base}/docs/design/counter"))
		.await
		.unwrap();
	cross_log!("stage: counter page loaded");
	// the sentence, since the locator is an exact innerText match
	page.find_text("You have clicked 0 times.").await;
	page.find_text("More").await;
	page.find_text("Less").await;

	// in-page navigability: the site's own links reach the blog and a post
	// (proves navigation, not just direct loads)
	cross_log!("stage: counter done");
	// `Page::click` re-queries on staleness: the client-side router
	// re-renders the page, invalidating held node references
	page.navigate(&base).await.unwrap();
	page.click("a[href='/blog']").await.unwrap();
	page.xpect_url(&format!("{base}/blog")).await;
	page.click("a[href='/blog/bevys-five-and-beets-alive']")
		.await
		.unwrap();
	page.xpect_url(&format!("{base}/blog/bevys-five-and-beets-alive"))
		.await;

	// the docs are unlisted: they serve (the counter above) but nobody
	// browsing finds them, so the landing page shows no link in, `/docs` asks
	// crawlers not to index it, and the sitemap leaves every docs page out
	cross_log!("stage: in-page nav done");
	page.navigate(&base).await.unwrap();
	page.evaluate_value(
		r#"[...document.querySelectorAll("a[href^='/docs']")]
			.filter(link => link.getClientRects().length > 0).length"#,
	)
	.await
	.unwrap()
	.as_u64()
	.xpect_eq(Some(0));
	page.evaluate_value(
		r#"fetch('/sitemap.xml').then(res => res.text()).then(text => text.includes('/docs'))"#,
	)
	.await
	.unwrap()
	.as_bool()
	.xpect_eq(Some(false));
	page.navigate(&format!("{base}/docs")).await.unwrap();
	page.evaluate_value(
		r#"document.querySelector('meta[name=robots]')?.content ?? null"#,
	)
	.await
	.unwrap()
	.as_str()
	.map(str::to_string)
	.xpect_eq(Some("noindex".to_string()));

	// the beacon runs on content pages, where the client error class
	// was originally reported
	cross_log!("stage: unlisted docs done");
	for path in ["/blog", "/blog/bevys-five-and-beets-alive"] {
		page.navigate(&format!("{base}{path}")).await.unwrap();
		page.find("body").await;
	}

	// asset sweep on a page that carries an image: every `/assets/`
	// reference fetches 200 through the page itself (a private bucket
	// handing out public urls turns each into a redirect to a 403)
	cross_log!("stage: blog pages done");
	page.navigate(&format!("{base}/blog/folk-technology"))
		.await
		.unwrap();
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

	cross_log!("stage: asset sweep done");

	// desktop layout: every table stays inside the page's measure column, a
	// table page's sidebar rail is exactly as wide as a prose page's, ie no
	// content widens the row around it, and every color swatch grows to hold
	// its wrapped label in the same type as its siblings. 1280 is inside the
	// rail's shrink range, 1920 past the column's measure cap.
	let mut layout_faults = Vec::new();
	for width in [1280, 1920] {
		page.set_viewport(width, 800).await.unwrap();
		page.navigate(&format!("{base}/docs/design/color_schemes"))
			.await
			.unwrap();
		page.find("main h1").await;
		let swatch_faults = page
			.evaluate_value(
				r#"(() => {
					const swatches = [...document.querySelectorAll('.color-box')];
					const size = swatch => getComputedStyle(swatch).fontSize;
					const expected = size(swatches[0]);
					return swatches.flatMap(swatch => {
						const label = swatch.textContent.trim();
						const faults = [];
						if (swatch.scrollHeight > swatch.clientHeight + 1)
							faults.push(`${label} overflows its box`);
						if (size(swatch) !== expected)
							faults.push(`${label} is ${size(swatch)}, not ${expected}`);
						return faults;
					});
				})()"#,
			)
			.await
			.unwrap();
		layout_faults.extend(
			swatch_faults
				.as_array()
				.unwrap()
				.iter()
				.filter_map(|fault| fault.as_str())
				.map(|fault| format!("{width}px color swatch {fault}")),
		);
		let prose =
			DesktopLayout::measure(&mut page, &base, "/docs/about").await;
		for path in ["/docs/design/table", "/docs/crates", "/docs/secrets"] {
			let layout = DesktopLayout::measure(&mut page, &base, path).await;
			if layout.sidebar != prose.sidebar {
				layout_faults.push(format!(
					"{width}px {path}: sidebar {} != prose sidebar {}",
					layout.sidebar, prose.sidebar
				));
			}
			layout_faults.extend(
				layout
					.escapes
					.into_iter()
					.map(|escape| format!("{width}px {path}: {escape}")),
			);
		}
	}
	layout_faults.xpect_empty();

	// mobile layout: no horizontal overflow at phone widths (the matcher
	// prints the offending elements on failure), and the opened sidebar
	// drawer overlays the content rather than squeezing it into overflow
	cross_log!("stage: desktop layout done");
	for width in [375, 320] {
		page.set_viewport(width, 812).await.unwrap();
		for path in ["/", "/blog", "/blog/bevys-five-and-beets-alive"] {
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

/// The desktop layout facts a page is judged by.
struct DesktopLayout {
	/// The sidebar rail's width in px.
	sidebar: f64,
	/// Each table whose visible box (its scroll wrapper) escapes the page's
	/// measure column, the extent of its `h1`.
	escapes: Vec<String>,
}

impl DesktopLayout {
	/// Navigate to `path` and measure its layout.
	async fn measure(page: &mut Page, base: &str, path: &str) -> Self {
		page.navigate(&format!("{base}{path}")).await.unwrap();
		page.find("main h1").await;
		let layout = page
			.evaluate_value(
				r#"(() => {
					const column = document.querySelector('main h1').getBoundingClientRect();
					return {
						sidebar: document.querySelector('#sidebar').getBoundingClientRect().width,
						escapes: [...document.querySelectorAll('main table')]
							.map(table => (table.closest('.table-scroll') ?? table).getBoundingClientRect())
							.filter(rect => rect.left < column.left - 0.5 || rect.right > column.right + 0.5)
							.map(rect => `table ${rect.left}..${rect.right} escapes column ${column.left}..${column.right}`),
					};
				})()"#,
			)
			.await
			.unwrap();
		Self {
			sidebar: layout["sidebar"].as_f64().unwrap(),
			escapes: layout["escapes"]
				.as_array()
				.unwrap()
				.iter()
				.map(|escape| escape.as_str().unwrap().to_string())
				.collect(),
		}
	}
}

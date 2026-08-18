//! The responsive sidebar drawer, driven in a real browser over a served
//! fixture: the [`PageHarness`] serves the real site chrome ([`SiteLayout`])
//! around a stand-in for the home page's "Mind your step" card, and the test
//! drives the [`MenuButton`] toggle at a phone viewport.
//!
//! Guards the drawer regression where the opened rail joined the container's
//! flex row instead of overlaying it: `<main>` was squeezed to a ~100px sliver
//! beside the 16rem rail, and the card's unshrinkable button row (centred by
//! the main column) spilled under the rail and past the viewport edge.
//!
//! Run: chromedriver + chromium on PATH, then
//! ```sh
//! cargo test --test sidebar_drawer \
//!   --features router,style,http_server,testing,webdriver -- --include-ignored
//! ```
beet::test_main!();

use beet::net::prelude::webdriver::*;
use beet::prelude::*;

/// The served site: the real [`SiteLayout`] chrome (app bar + [`MenuButton`],
/// [`RouteSidebar`], `<main>`) wrapping a home page carrying the card, plus a
/// couple of routes so the rail has links to show.
fn site() -> impl Bundle {
	(Router, BaseLayout::<SiteLayout>::default(), children![
		(render_action::fixed_func_route("", home), PageRoute),
		(
			render_action::fixed_func_route("docs", || rsx! { <p>"docs"</p> }),
			PageRoute
		),
		(
			render_action::fixed_func_route("blog", || rsx! { <p>"blog"</p> }),
			PageRoute
		),
	])
}

/// The home body, mirroring `site/routes/index.md`: a filled card whose
/// centred button row cannot shrink below ~220px — the widest unbreakable
/// content on the home page, and the piece that spilled.
fn home() -> impl Bundle {
	rsx! {
		<div {Classes::new([classes::CARD_FILLED])}>
			<h3>"🚧 Mind your step! 🚧"</h3>
			<p>"Beet is under construction."</p>
			<div {button_row()}>
				<Link href="https://github.com/mrchantey/beet" variant=ButtonVariant::Outlined>"GitHub"</Link>
				<Link href="/docs" variant=ButtonVariant::Filled>"Get Started"</Link>
			</div>
		</div>
	}
}

/// The card's centred action row (a `bx:style` flex row in the markdown
/// source): two buttons plus a gap, setting the card's min-content width.
fn button_row() -> impl Bundle {
	inline_class![
		(common_props::DisplayProp, Display::Flex),
		(common_props::JustifyContentProp, JustifyContent::Center),
		(common_props::AlignItemsProp, AlignItems::Center),
		(common_props::ColumnGapProp, Length::Rem(1.)),
	]
}

/// The document's `clientWidth`: the viewport minus any scrollbar, the width
/// every box is measured against.
async fn client_width(page: &Page) -> f32 {
	page.evaluate_value("document.documentElement.clientWidth")
		.await
		.unwrap()
		.as_f64()
		.unwrap() as f32
}

/// The `<main>` content column's bounding rect.
async fn main_rect(page: &Page) -> Rect {
	page.find("main").await.bounding_rect().await.unwrap()
}

#[beet_core::test(timeout_ms = 60_000)]
#[ignore = "smoketest"]
async fn drawer_overlays_content_on_mobile() {
	let mut page = PageHarness::serve(
		(
			RouterPlugin,
			material::MaterialStylePlugin::default(),
			// `SiteLayout`'s head/header read the site identity off this
			// resource; the live serve pipeline seeds it, a fixture must too
			|app: &mut App| {
				app.init_resource::<PackageConfig>();
			},
		),
		site(),
	)
	.await
	.unwrap();
	// an iphone-ish viewport, well below the collapse breakpoint
	page.set_viewport(375, 812).await.unwrap();
	page.goto("/").await.unwrap();

	// collapsed rail: the content column owns the viewport and the card fits
	let viewport = client_width(&page).await;
	page.xpect_no_horizontal_overflow().await;
	(main_rect(&page).await.width() >= viewport - 1.).xpect_true();

	// open the drawer
	page.find("#menu-button").await.click().await.unwrap();
	page.find("#sidebar")
		.await
		.xpect_attr("aria-hidden", "false")
		.await;

	// the open rail overlays the content: `<main>` keeps the full viewport ...
	let main = main_rect(&page).await;
	(main.min.x <= 1.).xpect_true();
	(main.width() >= viewport - 1.).xpect_true();
	// ... with the rail painted over its left edge, not flowed beside it ...
	let sidebar = page.find("#sidebar").await.bounding_rect().await.unwrap();
	(sidebar.min.x <= 1.).xpect_true();
	(sidebar.max.x > main.min.x + 100.).xpect_true();
	// ... and nothing spills: the card stays inside the viewport
	page.xpect_no_horizontal_overflow().await;
	let card = page
		.find(".card-filled")
		.await
		.bounding_rect()
		.await
		.unwrap();
	(card.min.x >= -1.).xpect_true();
	(card.max.x <= viewport + 1.).xpect_true();

	// the app bar sits above the drawer, so the toggle stays reachable to close
	page.find("#menu-button").await.click().await.unwrap();
	page.find("#sidebar")
		.await
		.xpect_attr("aria-hidden", "true")
		.await;

	// crossing back above the breakpoint restores the flowed rail beside the
	// content: the drawer is a narrow-viewport behavior only
	page.set_viewport(1280, 800).await.unwrap();
	page.find("#sidebar")
		.await
		.xpect_attr("aria-hidden", "false")
		.await;
	(main_rect(&page).await.min.x >= 200.).xpect_true();

	page.kill().await.unwrap();
}

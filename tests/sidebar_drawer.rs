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
/// [`RouteSidebar`], `<main>`, `Footer`) wrapping a home page carrying the
/// card, plus enough routes that the rail's nav list outgrows a phone viewport
/// (the shape that spilled over the footer).
fn site() -> impl Bundle {
	(
		Router,
		Layout::of::<SiteLayout>(),
		Children::spawn(bevy::ecs::spawn::SpawnIter(page_routes())),
	)
}

/// The route bundles: the home card page plus a flat run of stub pages, as
/// `fn` pointers so every route bundle is one type the spawn iterator unifies.
fn page_routes() -> impl Iterator<Item = impl Bundle> {
	fn home_body() -> Snippet { Snippet::from_bundle(home()) }
	fn stub_body() -> Snippet { Snippet::from_bundle(rsx! { <p>"page"</p> }) }
	std::iter::once(("".to_string(), home_body as fn() -> Snippet))
		.chain(
			std::iter::once("docs".to_string())
				.chain((0..24).map(|i| format!("post-{i}")))
				.map(|path| (path, stub_body as fn() -> Snippet)),
		)
		.map(|(path, page)| {
			(render_action::fixed_func_route(&path, page), PageRoute)
		})
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

/// The computed `display` of the header's nav-links cluster.
async fn nav_display(page: &Page) -> String {
	page.evaluate_value(
		"getComputedStyle(document.querySelector('.app-bar-nav')).display",
	)
	.await
	.unwrap()
	.as_str()
	.unwrap()
	.to_string()
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
	// the header's nav links hide rather than wrapping onto a second row:
	// below the breakpoint the drawer owns navigation
	nav_display(&page).await.xpect_eq("none");

	// open the drawer
	page.find("#menu-button").await.click().await.unwrap();
	page.find("#sidebar")
		.await
		.xpect_attr("aria-hidden", "false")
		.await;

	// the open rail overlays the content: `<main>` keeps the full viewport
	// (re-read, since a buggy drawer inflating the page also costs a scrollbar
	// width) ...
	let viewport = client_width(&page).await;
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

	// the drawer's long nav list scrolls within its own box ...
	page.evaluate_value(
		"(() => { const sb = document.getElementById('sidebar'); \
		 sb.scrollTop = 9999; return sb.scrollTop > 0; })()",
	)
	.await
	.unwrap()
	.as_bool()
	.unwrap()
	.xpect_true();
	// ... instead of spilling over the footer: the paint at the footer's left
	// half belongs to the footer, not an overflowing nav link ...
	page.evaluate_value(
		"(() => { const footer = document.querySelector('footer'); \
		 const rect = footer.getBoundingClientRect(); \
		 const hit = document.elementFromPoint(8, rect.top + rect.height / 2); \
		 return footer.contains(hit); })()",
	)
	.await
	.unwrap()
	.as_bool()
	.unwrap()
	.xpect_true();
	// ... and without inflating the page: opening the drawer on a short page
	// summons no vertical scrollbar
	page.evaluate_value(
		"document.documentElement.scrollHeight \
		 <= document.documentElement.clientHeight + 1",
	)
	.await
	.unwrap()
	.as_bool()
	.unwrap()
	.xpect_true();

	// the app bar sits above the drawer, so the toggle stays reachable to close
	page.find("#menu-button").await.click().await.unwrap();
	page.find("#sidebar")
		.await
		.xpect_attr("aria-hidden", "true")
		.await;

	// crossing back above the breakpoint restores the flowed rail beside the
	// content and the header nav links: the drawer is narrow-viewport only
	page.set_viewport(1280, 800).await.unwrap();
	page.find("#sidebar")
		.await
		.xpect_attr("aria-hidden", "false")
		.await;
	(main_rect(&page).await.min.x >= 200.).xpect_true();
	nav_display(&page).await.xpect_eq("flex");

	page.kill().await.unwrap();
}

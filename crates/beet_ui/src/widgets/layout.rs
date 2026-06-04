//! Page-layout widgets composing [`Head`], [`Header`], and [`Footer`] into
//! whole HTML pages.
//!
//! `<head>` is intentionally separate from the body chrome — these layouts
//! glue them together at the call site, and each layer only adds what it owns.
use beet_core::prelude::*;

/// Wraps an entire page, including `<head>` and `<body>`.
///
/// Slots: `head` (extra `<head>` content), default (page `<body>`).
#[scene]
pub fn HtmlDocument() -> impl Scene {
	rsx! {
		<html lang="en">
			<Head>
				<slot name="head"/>
			</Head>
			<body>
				<slot/>
			</body>
		</html>
	}
}

/// A standard HTML page: an [`HtmlDocument`] with a [`Header`] and [`Footer`]
/// around a body slot.
///
/// Slots: `head`, `header`, `header-nav`, `footer`, default (page body).
#[scene]
pub fn PageLayout() -> impl Scene {
	rsx! {
		<HtmlDocument>
			<slot name="head" slot="head"/>
			<div {Classes::new([classes::PAGE])}>
				<Header>
					<slot name="header"/>
					<slot name="header-nav" slot="nav"/>
				</Header>
				<slot/>
				<Footer>
					<slot name="footer"/>
				</Footer>
			</div>
		</HtmlDocument>
	}
}

/// Forces a page break when printing. Renders an empty element carrying the
/// [`classes::PAGE_BREAK`] class, styled by the `@media print` `page_break`
/// rule (`break-after: page`); a no-op on screen and non-web targets.
#[scene]
pub fn PageBreak() -> impl Scene {
	rsx! { <div {Classes::new([classes::PAGE_BREAK])}/> }
}

/// A [`PageLayout`] with a `<main>` content area for article-style pages.
///
/// Slots: `head`, `header`, `header-nav`, `footer`, default (main content).
#[scene]
pub fn ContentLayout() -> impl Scene {
	rsx! {
		<PageLayout>
			<slot name="head" slot="head"/>
			<slot name="header" slot="header"/>
			<slot name="header-nav" slot="header-nav"/>
			<slot name="footer" slot="footer"/>
			<main {Classes::new(["content-main"])}>
				<slot/>
			</main>
		</PageLayout>
	}
}

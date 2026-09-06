//! Page-layout widgets composing [`Head`], [`Header`], and [`Footer`] into
//! whole HTML pages.
//!
//! `<head>` is intentionally separate from the body chrome — these layouts
//! glue them together at the call site, and each layer only adds what it owns.
use crate::prelude::*;
use beet_core::prelude::*;

/// Wraps an entire page, including `<head>` and `<body>`.
///
/// Slots: `head` (extra `<head>` content), default (page `<body>`).
#[template]
pub fn HtmlDocument() -> impl Bundle {
	rsx! {
		<html lang="en">
			<Head>
				<Slot name="head"/>
			</Head>
			<body>
				<Slot/>
			</body>
		</html>
	}
}

/// A standard HTML page: an [`HtmlDocument`] with a [`Header`] and [`Footer`]
/// around a body slot.
///
/// Slots: `head`, `header`, `header-nav`, `footer`, default (page body).
#[template]
pub fn PageLayout() -> impl Bundle {
	rsx! {
		<HtmlDocument>
			<Slot name="head" slot="head"/>
			<div {Classes::new([classes::PAGE])}>
				<Header>
					<Slot name="header"/>
					<Slot name="header-nav" slot="nav"/>
				</Header>
				<Slot/>
				<Footer>
					<Slot name="footer"/>
				</Footer>
			</div>
		</HtmlDocument>
	}
}

/// Forces a page break when printing. Renders an empty element carrying the
/// [`classes::PAGE_BREAK`] class, styled by the `@media print` `page_break`
/// rule (`break-after: page`); a no-op on screen and non-web targets.
#[template]
pub fn PageBreak() -> impl Bundle {
	rsx! { <div {Classes::new([classes::PAGE_BREAK])}/> }
}

/// A [`PageLayout`] with a `<main>` content area for article-style pages.
///
/// Slots: `head`, `header`, `header-nav`, `footer`, default (main content).
#[template]
pub fn ContentLayout() -> impl Bundle {
	rsx! {
		<PageLayout>
			<Slot name="head" slot="head"/>
			<Slot name="header" slot="header"/>
			<Slot name="header-nav" slot="header-nav"/>
			<Slot name="footer" slot="footer"/>
			<main {Classes::new(["content-main"])}>
				<Slot/>
			</main>
		</PageLayout>
	}
}

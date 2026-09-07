//! The minimal document shell: an app's pages, themed, with no chrome.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::material::colors;
use beet_ui::prelude::*;
// the head-chrome widget, imported by name so the tag resolves regardless of
// which glob (`crate::prelude` vs `beet_ui::prelude`) also defines a `Reset`.
use beet_ui::prelude::Reset;

/// An html document whose body is one full-height flex column around the page,
/// the shell an app wraps its routes in when it wants the theme but none of
/// [`SiteLayout`]'s chrome.
///
/// Themed exactly as [`SiteLayout`] is: [`PageClasses`] pins the session scheme
/// per request, so `--color-scheme` reaches a page over the same two hops, and
/// the web-only head chrome (the [`Stylesheet`] CSS bake, preflight/reset, the
/// color-scheme script) is emitted only for the HTML target, since `<head>` is
/// `display: none` in the terminal and baking the rule set there is pure cost.
///
/// What it deliberately lacks is chrome: no header, sidebar or footer, and no
/// `<main>` — the column stretches its children to the full width, so a page
/// pins a composer to the bottom or fills the window with a table as it likes.
/// A page wanting the padded, measure-capped content column authors its own
/// `<main>`, which the shipped rules style.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so
/// an app wraps its routes in it with `<Router {Layout{template:"AppShell"}}>`.
#[template(system)]
pub fn AppShell(
	stack: Res<RequestContextStack>,
	theme: Res<Theme>,
) -> impl Bundle {
	let cx = stack.current();
	let body_classes = PageClasses::resolve(cx.parts(), &theme);
	let html_head = cx.parts().accepts(MediaType::Html).then(|| {
		rsx! {
			<Preflight/>
			<Reset/>
			<Stylesheet/>
			<ColorSchemeScript/>
		}
	});
	rsx! {
		<html lang="en">
			<head>{html_head}</head>
			<body {(body_classes, page_column())}>
				<Slot/>
			</body>
		</html>
	}
}

/// The page body's column: a viewport-height flex column tinted with the surface
/// palette, so content grows and a footer row pins to the bottom.
///
/// The shipped `.page` rule expresses the same column, but only once a rule set
/// is registered ([`MaterialStylePlugin`](beet_ui::prelude::material::MaterialStylePlugin));
/// declaring it inline keeps a bare app laid out correctly either way. Cascade
/// styling (`inline_class!`), since `resolve_styles` rebuilds every node's
/// `LayoutStyle` from the cascade, which would clobber a set component.
fn page_column() -> impl Bundle {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Vertical
		),
		// stretch children across the full width, so a table and a footer row
		// (and its top-border separator) span the terminal
		(
			style::common_props::AlignItemsProp,
			style::AlignItems::Stretch
		),
		(
			style::common_props::Height,
			style::Length::ViewportHeight(100.)
		),
		Declaration::token(
			style::common_props::BackgroundColor,
			colors::Surface
		),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::OnSurface
		),
	]
}

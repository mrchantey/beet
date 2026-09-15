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
/// One slot, `head`: extra head content appended to the chrome (a page's
/// `<Wasm>` launch), for a layout wrapping this shell to fill.
///
/// # Scrolling
///
/// The body is a *fixed* `100vh` column, so the content region is what scrolls
/// rather than the page: [`app_shell_rules`] gives a `<main>` inside this shell
/// `overflow-y: auto`, the same `flex-grow` + `auto` recipe a thread transcript
/// uses for its own region. A page that wants the scroll elsewhere (a thread
/// pinning its composer) simply scrolls that element instead, and one that
/// authors no `<main>` is clipped to the viewport, which is what a fixed shell
/// means.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so
/// an app wraps its routes in it with `<Router {Layout{template:"AppShell"}}>`.
#[template(system)]
pub fn AppShell(
	stack: Res<RequestContextStack>,
	theme: Res<Theme>,
) -> impl Bundle {
	let cx = stack.current();
	let mut body_classes = PageClasses::resolve(cx.parts(), &theme);
	body_classes.insert_class(APP_SHELL);
	// the charset first: the response names none, and a browser reading utf-8
	// as latin-1 renders every box-drawing guide and arrow as mojibake
	let html_head = cx.parts().accepts(MediaType::Html).then(|| {
		rsx! {
			<meta charset="utf-8"/>
			<meta name="viewport" content="width=device-width, initial-scale=1"/>
			<Preflight/>
			<Reset/>
			<Stylesheet/>
			<ColorSchemeScript/>
		}
	});
	rsx! {
		<html lang="en">
			<head>
				{html_head}
				<Slot name="head"/>
			</head>
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

/// The shell's own class, marking a body whose height is the viewport's.
///
/// Scopes [`app_shell_rules`] to this shell: the shipped `.page` is a
/// *document-flow* body (`min-height: 100vh`) whose content grows and whose
/// page-host scrollport scrolls, and giving its `<main>` a scroll region too
/// would put two scrollbars on every site page.
pub const APP_SHELL: ClassName = ClassName::new_static("app-shell");

/// The shell's scroll region: a `<main>` inside an [`AppShell`] scrolls its own
/// content, since the shell's body cannot grow past the viewport.
///
/// Contributed by [`RouterPlugin`](crate::prelude::RouterPlugin) through
/// [`RuleSet::extend_rules`], the way the card-stack rules are, so it composes
/// with the material set and overrides `main` on a tie.
pub(crate) fn app_shell_rules() -> Vec<Rule> {
	vec![
		Rule::new()
			.with_selector(Selector::descendant(
				Selector::class(APP_SHELL),
				Selector::tag("main"),
			))
			.with_value(
				style::common_props::OverflowYProp,
				style::Overflow::Auto,
			),
	]
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// A router world seeded with the request-scoped facts the shell reads.
	fn shell_world(accept: MediaType) -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.init_resource::<PackageConfig>();
		let route = world
			.spawn((
				render_action::fixed_func_route("", || rsx! { <p>"body"</p> }),
				PageRoute,
			))
			.flush();
		let mut parts = RequestParts::get("");
		parts.headers_mut().set::<header::Accept>(vec![accept]);
		world
			.resource_mut::<RequestContextStack>()
			.push(RequestContext::new(parts, route, route, route));
		world
	}

	/// Render `<AppShell>` with a page body and a head-slot fill.
	fn render(accept: MediaType) -> String {
		let mut world = shell_world(accept);
		let entity = world
			.spawn_template(rsx! {
				<AppShell>
					<meta name="page-launch" slot="head"/>
					<p>"page body"</p>
				</AppShell>
			})
			.unwrap()
			.id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	/// The web head declares its charset before anything else, so a served
	/// page's box-drawing guides never read as latin-1, and a wrapping
	/// layout's head-slot content lands in it; the terminal's head is bare.
	#[beet_core::test]
	fn the_web_head_names_its_charset_and_takes_the_slot() {
		let html = render(MediaType::Html);
		html.as_str()
			.xpect_contains("<head><meta charset=\"utf-8\" />")
			.xpect_contains("<meta name=\"page-launch\" />")
			.xpect_contains("<p>page body</p>");
		render(MediaType::Text)
			.xnot()
			.xpect_contains("charset")
			.xpect_contains("<meta name=\"page-launch\" />");
	}
}

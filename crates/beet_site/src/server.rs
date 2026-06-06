use crate::prelude::*;
use beet::prelude::*;

/// The site's render substrate, shared by the web and terminal entry points.
///
/// Adds the router observers and charcell pipeline (via [`RouterPlugin`]) plus
/// the Material style rule set that both the web [`Stylesheet`] and the charcell
/// renderer read.
pub fn server_plugin(app: &mut App) {
	app.add_plugins((
		MinimalPlugins,
		RouterPlugin,
		ServerPlugin,
		material::MaterialStylePlugin::new(THEME_COLOR),
	));
	// site-local layout rule, see `design_row_rule`. The landing-page hero uses
	// `inline_class!` instead, since its layout is a single-use one-off.
	let mut rules = app.world_mut().get_resource_or_init::<RuleSet>();
	rules.insert_rule(design_row_rule());
}

/// Site-local class for the design showcase rows that lay widget variants out
/// side by side. A const so the rule and the pages that emit it share one name.
pub const DESIGN_ROW: ClassName = ClassName::new_static("design-row");
/// A horizontal flex row with a gap, for the design showcase pages that lay out
/// widget variants side by side.
///
/// Expressed as design tokens rather than a raw `<style>` so it spaces items in
/// both the web and terminal targets, mirroring the library `app-bar-nav` rule.
fn design_row_rule() -> Rule {
	use style::AlignItems;
	use style::Display;
	use style::FlexWrap;
	use style::Length;
	use style::common_props;
	// the `Length` row/column gap props serialize to valid CSS *and* drive the
	// charcell flex layout, so one value spaces items on both targets.
	Rule::new()
		.with_selector(Selector::class(DESIGN_ROW))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexWrapProp, FlexWrap::Wrap)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
		.with_value(common_props::RowGapProp, Length::Rem(1.0))
}

/// Every site route: the page collection plus the docs and blog collections,
/// each grouped under a [`BlobStore`] rooted at its markdown source directory so
/// their [`BlobScene`] routes can read their content.
pub fn beet_site_endpoints() -> impl Bundle {
	children![
		pages_routes(),
		(content_store("docs"), docs_routes()),
		(content_store("blog"), blog_routes()),
	]
}

/// A [`BlobStore`] rooted at `crates/beet_site/src/<dir>`, the source directory
/// of a markdown collection.
fn content_store(dir: &str) -> BlobStore {
	let root =
		AbsPathBuf::new_workspace_rel(format!("crates/beet_site/src/{dir}"))
			.unwrap();
	BlobStore::new(FsStore::new(root))
}

/// The site router.
///
/// The batteries-included [`default_router`] (adding `/app-info` and
/// `POST /analytics`) wrapped in the global [`BeetDocumentShell`] via the
/// [`DocumentShell`] render middleware, so every route's body is placed into
/// the shell's `<main>`.
pub fn beet_site_router() -> impl Bundle {
	(
		default_router(),
		beet_site_endpoints(),
		DocumentShell::<BeetDocumentShell>::default(),
	)
}

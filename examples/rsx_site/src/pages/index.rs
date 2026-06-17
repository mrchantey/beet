use crate::prelude::*;
use beet::prelude::*;

/// The site landing page, placed into the [`BeetLayout`] `<main>`.
///
/// One-off layout (the centered hero column, the readable body measure, the
/// centered button row) is declared with `inline_class!` right on each element,
/// extracted into the small helpers below so the markup stays readable. This is
/// the core `rsx!` + `inline_class!` demo.
pub fn get() -> impl Bundle {
	rsx! {
		<div {hero_column()}>
			<h1 {Classes::new([classes::TEXT_DISPLAY_MEDIUM])}>"Beet"</h1>
			<p {Classes::new([classes::TEXT_TITLE_LARGE])}>
				<b>"A malleable application framework"</b>
			</p>
			<div {block_measure()}>
				<p {Classes::new([classes::TEXT_BODY_LARGE, classes::TEXT_LEFT])}>
					"This is the typed (rsx!) authoring path: pages are "
					<code>"pub fn get() -> impl Bundle"</code>
					" returning markup. See the "
					<Link href=routes::counter() variant=ButtonVariant::Text>"counter"</Link>
					" for native reactivity, or the "
					<Link href=routes::buttons() variant=ButtonVariant::Text>"buttons"</Link>
					" for the widget set."
				</p>
			</div>
			<div {button_row()}>
				<Link href="https://github.com/mrchantey/beet" variant=ButtonVariant::Outlined>"GitHub"</Link>
				<Link href=routes::counter() variant=ButtonVariant::Filled>"Counter"</Link>
			</div>
		</div>
	}
}

/// The centered hero column: a vertical flex stack, centered, with vertical
/// rhythm between its blocks.
fn hero_column() -> OnSpawn {
	inline_class![
		(common_props::DisplayProp, style::Display::Flex),
		(common_props::FlexDirectionProp, style::Direction::Vertical),
		(common_props::AlignItemsProp, style::AlignItems::Center),
		(common_props::TextAlignProp, style::TextAlign::Center),
		(common_props::RowGapProp, Length::Rem(1.5)),
	]
}

/// Constrain a hero block to a readable measure.
fn block_measure() -> OnSpawn {
	inline_class![(common_props::MaxWidth, Length::Rem(34.))]
}

/// The centered call-to-action button row.
fn button_row() -> OnSpawn {
	inline_class![
		(common_props::DisplayProp, style::Display::Flex),
		(common_props::JustifyContentProp, style::JustifyContent::Center),
		(common_props::AlignItemsProp, style::AlignItems::Center),
		(common_props::ColumnGapProp, Length::Rem(1.)),
	]
}

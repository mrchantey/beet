use crate::prelude::*;
use beet::prelude::*;

/// The site landing page, placed into the [`BeetDocumentShell`] `<main>`.
///
/// One-off layout (the centered hero column, the card and body max-widths, the
/// centered button row) is declared with `inline_class!` right on each element,
/// extracted into the small helpers below so the markup stays readable.
pub fn get() -> impl Scene {
	rsx! {
		<div {hero_column()}>
			<h1 {Classes::new([classes::TEXT_DISPLAY_MEDIUM])}>"Beet"</h1>
			<p {Classes::new([classes::TEXT_TITLE_LARGE])}>
				<b>"A malleable application framework"</b>
			</p>
			<div {block_measure()}>
				<div {Classes::new([classes::CARD_FILLED])}>
					<h3>"🚧 Mind your step! 🚧"</h3>
					<p>
						"Beet is under construction. If this project is of interest please come and say hi in the "
						<a href="https://discord.gg/DcURUQCXtx">"Beetmash Discord Server"</a>
						"."
					</p>
					<div {button_row()}>
						<Link href="https://github.com/mrchantey/beet" variant=ButtonVariant::Outlined>"GitHub"</Link>
						<Link href=routes::blog::index() variant=ButtonVariant::Filled>"Blog"</Link>
					</div>
				</div>
			</div>
			<div {block_measure()}>
				<p {Classes::new([classes::TEXT_BODY_LARGE, classes::TEXT_LEFT])}>
					"Beet is a framework for building user-modifiable applications, like Smalltalk or HyperCard. Everything from the CLI to client applications is a "
					<a href="https://bevy.org">"Bevy App"</a>
					", and all structure and behavior is written in Entity Component System architecture."
				</p>
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
		(common_props::ColumnGapProp, Length::Rem(1.)),
	]
}

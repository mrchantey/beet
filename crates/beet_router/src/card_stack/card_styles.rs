//! The layout classes for a stack of cards and their token-based rules.
//!
//! A [`CardDeck`](crate::prelude::CardDeck) frames each card as a full-viewport
//! flex column plus two body layouts (a centered title card, a heading over a row
//! of columns). The rules are token-based like the rest of beet_ui's set, so a
//! card renders the same in a browser and on the charcell terminal: the web
//! full-height comes from the screen-gated [`card_fill_viewport`] (the terminal
//! grows to fit its content), mirroring the material `page` rules.
//!
//! [`CardStackPlugin`](crate::prelude::CardStackPlugin) contributes [`card_rules`]
//! by extending the shared [`RuleSet`], the same way the material/style plugins
//! add theirs; it is added after `MaterialStylePlugin`, so a card rule wins a
//! cascade tie with the material set, and other plugins can extend the set again
//! after to refine these.
#![cfg_attr(rustfmt, rustfmt_skip)]
use beet_core::prelude::*;
use beet_ui::prelude::*;
use beet_ui::prelude::style::common_props;
// the layout value enums (`Display`, `Direction`, `Length`, …) reached via the
// `style::*` glob, mirroring beet_ui's own rule modules.
use beet_ui::prelude::style::*;

/// A card frame — the per-card full-viewport flex column.
pub const CARD: ClassName = ClassName::new_static("card");
/// A card whose single content block is centered on both axes (title cards).
pub const CARD_CENTER: ClassName = ClassName::new_static("card-center");
/// A card laid out as a heading band over a row of content columns.
pub const CARD_CONTENT: ClassName = ClassName::new_static("card-content");
/// The heading band of a [`CARD_CONTENT`] card.
pub const CARD_HEADING: ClassName = ClassName::new_static("card-heading");
/// The growing column row of a [`CARD_CONTENT`] card.
pub const CARD_COLUMNS: ClassName = ClassName::new_static("card-columns");
/// A single column within [`CARD_COLUMNS`], sharing the row width evenly.
pub const CARD_COLUMN: ClassName = ClassName::new_static("card-column");

/// The card-stack layout rules: the per-card frame and its two body layouts.
///
/// Contributed by [`CardStackPlugin`](crate::prelude::CardStackPlugin) via
/// [`RuleSet::extend_rules`], so they compose with (and override on a tie) the
/// material set, and stay extensible by later plugins.
pub fn card_rules() -> Vec<Rule> {
	vec![
		card(),
		card_center(),
		card_content(),
		card_heading(),
		card_columns(),
		card_column(),
		// web-only override, gated behind `@media screen` (ignored by charcell)
		card_fill_viewport(),
	]
}

/// Card frame - a full-width flex column with comfortable padding, so each card
/// fills the viewport (web, via [`card_fill_viewport`]) or grows to fit (the
/// terminal).
///
/// `align-items: stretch` is the CSS flexbox default but *not* beet's
/// ([`AlignItems`] defaults to `Start`), so it is set explicitly here: without it
/// the charcell terminal hugs the body to its content and pins it to the left,
/// while the browser stretches it full-width — a card body that centres on the
/// web (eg `card-center`) would sit left in the terminal. Setting it keeps the
/// slide horizontally consistent across both targets (a no-op on the web, which
/// already defaults to stretch).
pub fn card() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::AlignItemsProp, AlignItems::Stretch)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(2.0)))
}

/// On the web, a card fills at least the viewport height, the deck's full-screen
/// frame; the terminal ignores this media-gated rule and grows to fit instead.
pub fn card_fill_viewport() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::class(CARD))
		.with_value(common_props::MinHeight, Length::ViewportHeight(100.))
}

/// Centered card body (title cards) - grows to fill the card and centers its
/// children on both axes, with text centered.
pub fn card_center() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_CENTER))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::FlexGrowProp, 1u32)
		.with_value(common_props::JustifyContentProp, JustifyContent::Center)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::TextAlignProp, TextAlign::Center)
}

/// Content card body - a flex column filling the card: the heading band on top,
/// the column row below growing to fill.
pub fn card_content() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_CONTENT))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::FlexGrowProp, 1u32)
}

/// Card heading band - the heading row, holding its natural height (no growth).
pub fn card_heading() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_HEADING))
		.with_value(common_props::FlexGrowProp, 0u32)
}

/// Card column row - a flex row growing to fill the height beneath the heading,
/// with a comfortable gap between columns.
pub fn card_columns() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_COLUMNS))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Horizontal)
		.with_value(common_props::FlexGrowProp, 1u32)
		.with_value(common_props::ColumnGapProp, Length::Rem(2.0))
}

/// Card column - shares the row width evenly with its siblings; a single column
/// fills the row. Empty columns never reach the tree: `ContentLayout` only emits
/// a `card-column` for content the author supplies (see its template), so there
/// is nothing to collapse.
pub fn card_column() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_COLUMN))
		.with_value(common_props::FlexGrowProp, 1u32)
}

//! Site-local style: the class vocabulary and the rules layered onto the
//! library Material set.
use beet::prelude::*;

/// The site's own class names, layered on the library
/// [`classes`](beet::prelude::classes).
///
/// Note: inside `rsx!` a bare `classes::` always resolves to the *library*
/// module (the macro injects `beet_ui::prelude::*`), so a site-local class is
/// referenced by its full path, eg `crate::style::classes::DESIGN_ROW`.
pub mod classes {
	use beet::prelude::ClassName;

	/// Site-local class for the design showcase rows that lay widget variants
	/// out side by side.
	pub const DESIGN_ROW: ClassName = ClassName::new_static("design-row");
}

/// A horizontal flex row with a gap, for the design showcase pages that lay out
/// widget variants side by side, styling [`classes::DESIGN_ROW`].
///
/// Expressed as design tokens rather than a raw `<style>` so it spaces items in
/// both the web and terminal targets, mirroring the library `app-bar-nav` rule.
pub fn design_row_rule() -> Rule {
	use style::AlignItems;
	use style::Display;
	use style::FlexWrap;
	use style::Length;
	use style::common_props;
	// the `Length` row/column gap props serialize to valid CSS *and* drive the
	// charcell flex layout, so one value spaces items on both targets.
	Rule::new()
		.with_selector(Selector::class(classes::DESIGN_ROW))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexWrapProp, FlexWrap::Wrap)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
		.with_value(common_props::RowGapProp, Length::Rem(1.0))
}

//! Renders a boxed element once to the terminal and exits.
//!
//! The box model is authored through the cascade (`inline_class!`), since style
//! resolution rebuilds an element's [`BoxStyle`] from its rules, clobbering a
//! hand-attached component.
use beet_core::prelude::*;
use beet_ui::prelude::style::common_props::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use bevy::math::UVec2;

fn main() {
	let size = terminal_ext::size();
	Buffer::render_oneshot_sized(UVec2::new(size.x, size.y / 2), setup())
		.xprint();
}

fn setup() -> impl Bundle {
	rsx! {<div {boxed()}>"hello world!"</div>}
}

/// A margin, a one cell border and padding around the content.
fn boxed() -> impl Bundle {
	inline_class![
		(MarginProp, Spacing::all(Length::Rem(3.))),
		(BorderTopWidth, Length::Rem(1.)),
		(BorderRightWidth, Length::Rem(1.)),
		(BorderBottomWidth, Length::Rem(1.)),
		(BorderLeftWidth, Length::Rem(1.)),
		(Padding, Spacing::all(Length::Rem(3.))),
	]
}

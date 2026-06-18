use beet_core::prelude::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use bevy::math::UVec2;

fn main() {
	let size = terminal_ext::size();
	Buffer::render_oneshot_sized(UVec2::new(size.x, size.y / 2), setup())
		.xprint();
}

fn setup() -> impl Bundle {
	(
		rsx! {<div>"hello world!"</div>},
		VisualStyle::default(),
		BoxStyle::default()
			.with_margin(Spacing::all(Length::Rem(3.)))
			.with_border(Spacing::all(Length::Rem(1.)))
			.with_padding(Spacing::all(Length::Rem(3.))),
	)
}

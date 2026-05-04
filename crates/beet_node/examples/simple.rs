use beet_core::prelude::*;
use beet_node::prelude::style::*;
use beet_node::prelude::*;
use beet_node::*;


fn main() {
	RenderCharcell::default()
		.render_oneshot(setup())
		.unwrap()
		.render()
		.trim_lines()
		.xprint();
}



fn setup() -> impl Bundle {
	(
		rsx! {"hello world!"},
		VisualStyle::default(),
		LayoutStyle::default()
			.with_margin(Spacing::all(Length::Rem(3.)))
			.with_border(Spacing::all(Length::Rem(1.)))
			.with_padding(Spacing::all(Length::Rem(3.))),
	)
}

use beet_core::prelude::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use beet_ui::*;


fn main() {
	CharcellRenderer::default()
		.halved()
		.render_oneshot(setup())
		.unwrap()
		.render()
		// .trim_lines()
		.xprint();
}



fn setup() -> impl Bundle {
	(
		rsx! {"hello world!"},
		VisualStyle::default(),
		BoxStyle::default()
			.with_margin(Spacing::all(Length::Rem(3.)))
			.with_border(Spacing::all(Length::Rem(1.)))
			.with_padding(Spacing::all(Length::Rem(3.))),
	)
}

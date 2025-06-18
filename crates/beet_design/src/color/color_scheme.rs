use crate::prelude::*;
use material_colors::color::Argb;
use material_colors::theme::Theme;
use material_colors::theme::ThemeBuilder;

#[template]
pub fn ColorScheme(theme: Theme) -> impl Bundle {
	let css = ThemeToCss::default().map(&theme);
	(
		NodeTag::new("style"),
		StyleScope::Global,
		ElementNode::open(),
		children![TextNode::new(css)],
	)
}

impl ColorScheme {
	pub fn new_from_color(r: u8, g: u8, b: u8) -> Self {
		let theme = ThemeBuilder::with_source(Argb::new(255, r, g, b))
			// .variant(Variant::TonalSpot)
			// .color_match(false)
			.build();
		Self { theme }
	}
}


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		// let color_scheme = ColorScheme::new_from_color(255, 0, 0);
		// println!("{:#?}", color_scheme);
	}
}

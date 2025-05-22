use crate::prelude::*;
use beet_rsx::prelude::*;
use material_colors::color::Argb;
use material_colors::theme::Theme;
use material_colors::theme::ThemeBuilder;


#[derive(Debug, derive_template)]
pub struct ColorScheme {
	theme: Theme,
}

fn color_scheme(props: ColorScheme) -> WebNode {
	// Theme

	let css = ThemeToCss::default().map(&props.theme);

	Style::new(css)
		.with_directive(TemplateDirectiveEnum::StyleScope(StyleScope::Global))
		.into_node()
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

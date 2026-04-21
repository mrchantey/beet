use super::*;
use crate::prelude::*;
use beet_core::prelude::*;



token!(
	Color,
	PRIMARY_BACKGROUND,
	PRIMARY_BACKGROUND_META,
	"primary-background",
	"Primary Background Color",
	"The color of surfaces."
);
token!(
	Color,
	SURFACE_TINT,
	SURFACE_TINT_META,
	"surface-tint",
	"Tint Background Color",
	"Tint surfaces."
);
token!(
	Color,
	PRIMARY_FOREGROUND,
	PRIMARY_FOREGROUND_META,
	"foreground",
	"Foreground Color",
	"The color of text and other foreground elements."
);


impl AsTokenValue for Color {
	fn category() -> &'static str { "color" }
	fn to_css_value(&self) -> String {
		let this = self.to_srgba();
		format!(
			"rgba({}, {}, {}, {})",
			(this.red * 255.0).round() as u8,
			(this.green * 255.0).round() as u8,
			(this.blue * 255.0).round() as u8,
			this.alpha
		)
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_name() {
		println!("Token Name: {}", PRIMARY_BACKGROUND);
	}
}

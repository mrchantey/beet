#![cfg_attr(rustfmt, rustfmt_skip)]
use beet_core::prelude::*;
use crate::style::*;
use crate::prelude::*;

token!(Color, PRIMARY, "primary");
token!(Color, ON_PRIMARY, "on-primary");
token!(Color, PRIMARY_CONTAINER, "primary-container");
token!(Color, ON_PRIMARY_CONTAINER, "on-primary-container");
token!(Color, INVERSE_PRIMARY, "inverse-primary");
token!(Color, PRIMARY_FIXED, "primary-fixed");
token!(Color, PRIMARY_FIXED_DIM, "primary-fixed-dim");
token!(Color, ON_PRIMARY_FIXED, "on-primary-fixed");
token!(Color, ON_PRIMARY_FIXED_VARIANT, "on-primary-fixed-variant");
token!(Color, SECONDARY, "secondary");
token!(Color, ON_SECONDARY, "on-secondary");
token!(Color, SECONDARY_CONTAINER, "secondary-container");
token!(Color, ON_SECONDARY_CONTAINER, "on-secondary-container");
token!(Color, SECONDARY_FIXED, "secondary-fixed");
token!(Color, SECONDARY_FIXED_DIM, "secondary-fixed-dim");
token!(Color, ON_SECONDARY_FIXED, "on-secondary-fixed");
token!(Color, ON_SECONDARY_FIXED_VARIANT,"on-secondary-fixed-variant");
token!(Color, TERTIARY, "tertiary");
token!(Color, ON_TERTIARY, "on-tertiary");
token!(Color, TERTIARY_CONTAINER, "tertiary-container");
token!(Color, ON_TERTIARY_CONTAINER, "on-tertiary-container");
token!(Color, TERTIARY_FIXED, "tertiary-fixed");
token!(Color, TERTIARY_FIXED_DIM, "tertiary-fixed-dim");
token!(Color, ON_TERTIARY_FIXED, "on-tertiary-fixed");
token!(Color, ON_TERTIARY_FIXED_VARIANT,"on-tertiary-fixed-variant");
token!(Color, ERROR, "error");
token!(Color, ON_ERROR, "on-error");
token!(Color, ERROR_CONTAINER, "error-container");
token!(Color, ON_ERROR_CONTAINER, "on-error-container");
token!(Color, SURFACE_DIM, "surface-dim");
token!(Color, SURFACE, "surface");
token!(Color, SURFACE_TINT, "surface-tint");
token!(Color, SURFACE_BRIGHT, "surface-bright");
token!(Color, SURFACE_CONTAINER_LOWEST, "surface-container-lowest");
token!(Color, SURFACE_CONTAINER_LOW, "surface-container-low");
token!(Color, SURFACE_CONTAINER, "surface-container");
token!(Color, SURFACE_CONTAINER_HIGH, "surface-container-high");
token!(Color, SURFACE_CONTAINER_HIGHEST,"surface-container-highest");
token!(Color, ON_SURFACE, "on-surface");
token!(Color, ON_SURFACE_VARIANT, "on-surface-variant");
token!(Color, OUTLINE, "outline");
token!(Color, OUTLINE_VARIANT, "outline-variant");
token!(Color, INVERSE_SURFACE, "inverse-surface");
token!(Color, INVERSE_ON_SURFACE, "inverse-on-surface");
token!(Color, SURFACE_VARIANT, "surface-variant");
token!(Color, BACKGROUND, "background");
token!(Color, ON_BACKGROUND, "on-background");
token!(Color, SHADOW, "shadow");
token!(Color, SCRIM, "scrim");




impl CssValue for Color {
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
		println!("Token Name: {}", PRIMARY);
	}
}

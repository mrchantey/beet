use beet_core::prelude::*;
use bevy::color::Color;
use material_colors::color::Argb;
use material_colors::scheme::Scheme;
use material_colors::theme::Theme;
use material_colors::theme::ThemeBuilder;


#[derive(Get)]
pub struct MaterialTheme {
	theme: Theme,
}
#[extend::ext(name=MaterialColorExt)]
pub impl Color {
	/// Converts the color to an sRGBA representation with 8 bits per channel.
	fn to_argb(&self) -> Argb {
		let srgba = self.to_srgba_u8();
		Argb::new(srgba.alpha, srgba.red, srgba.green, srgba.blue)
	}
}


impl MaterialTheme {
	pub fn new(color: impl Into<Color>) -> Self {
		let theme = ThemeBuilder::with_source(color.into().to_argb())
			// .variant(Variant::TonalSpot)
			// .color_match(false)
			.build();
		Self { theme }
	}
	pub fn new_with_theme(theme: Theme) -> Self { Self { theme } }
}

pub struct ThemeToCss {
	pub prefix: SmolStr,
	/// The global class applied to the document in light mode
	/// This must be kept in sync with initColorScheme.js
	pub light_class: SmolStr,
	/// The global class applied to the document in dark mode
	/// This must be kept in sync with initColorScheme.js
	pub dark_class: SmolStr,
}

impl Default for ThemeToCss {
	fn default() -> Self {
		Self {
			prefix: Self::DEFAULT_PREFIX.into(),
			light_class: Self::DEFAULT_LIGHT_CLASS.into(),
			dark_class: Self::DEFAULT_DARK_CLASS.into(),
		}
	}
}


impl ThemeToCss {
	pub const DEFAULT_PREFIX: &'static str = "bt-color";
	pub const DEFAULT_LIGHT_CLASS: &'static str = "scheme-light";
	pub const DEFAULT_DARK_CLASS: &'static str = "scheme-dark";

	pub fn map(&self, theme: &Theme) -> String {
		let light = self.scheme_to_css(&self.light_class, &theme.schemes.light);
		let dark = self.scheme_to_css(&self.dark_class, &theme.schemes.dark);
		format!(
			r#"
		{light}
		{dark}
		"#
		)
	}

	fn scheme_to_css(&self, class: &str, scheme: &Scheme) -> String {
		let prefix = &self.prefix;
		let Scheme {
			primary,
			on_primary,
			primary_container,
			on_primary_container,
			inverse_primary,
			primary_fixed,
			primary_fixed_dim,
			on_primary_fixed,
			on_primary_fixed_variant,
			secondary,
			on_secondary,
			secondary_container,
			on_secondary_container,
			secondary_fixed,
			secondary_fixed_dim,
			on_secondary_fixed,
			on_secondary_fixed_variant,
			tertiary,
			on_tertiary,
			tertiary_container,
			on_tertiary_container,
			tertiary_fixed,
			tertiary_fixed_dim,
			on_tertiary_fixed,
			on_tertiary_fixed_variant,
			error,
			on_error,
			error_container,
			on_error_container,
			surface_dim,
			surface,
			surface_tint,
			surface_bright,
			surface_container_lowest,
			surface_container_low,
			surface_container,
			surface_container_high,
			surface_container_highest,
			on_surface,
			on_surface_variant,
			outline,
			outline_variant,
			inverse_surface,
			inverse_on_surface,
			surface_variant,
			background,
			on_background,
			shadow,
			scrim,
		} = scheme;

		format!(
			r#"
.{class} {{
	--{prefix}-primary: {primary};
	--{prefix}-on-primary: {on_primary};
	--{prefix}-primary-container: {primary_container};
	--{prefix}-on-primary-container: {on_primary_container};
	--{prefix}-inverse-primary: {inverse_primary};
	--{prefix}-primary-fixed: {primary_fixed};
	--{prefix}-primary-fixed-dim: {primary_fixed_dim};
	--{prefix}-on-primary-fixed: {on_primary_fixed};
	--{prefix}-on-primary-fixed-variant: {on_primary_fixed_variant};
	--{prefix}-secondary: {secondary};
	--{prefix}-on-secondary: {on_secondary};
	--{prefix}-secondary-container: {secondary_container};
	--{prefix}-on-secondary-container: {on_secondary_container};
	--{prefix}-secondary-fixed: {secondary_fixed};
	--{prefix}-secondary-fixed-dim: {secondary_fixed_dim};
	--{prefix}-on-secondary-fixed: {on_secondary_fixed};
	--{prefix}-on-secondary-fixed-variant: {on_secondary_fixed_variant};
	--{prefix}-tertiary: {tertiary};
	--{prefix}-on-tertiary: {on_tertiary};
	--{prefix}-tertiary-container: {tertiary_container};
	--{prefix}-on-tertiary-container: {on_tertiary_container};
	--{prefix}-tertiary-fixed: {tertiary_fixed};
	--{prefix}-tertiary-fixed-dim: {tertiary_fixed_dim};
	--{prefix}-on-tertiary-fixed: {on_tertiary_fixed};
	--{prefix}-on-tertiary-fixed-variant: {on_tertiary_fixed_variant};
	--{prefix}-error: {error};
	--{prefix}-on-error: {on_error};
	--{prefix}-error-container: {error_container};
	--{prefix}-on-error-container: {on_error_container};
	--{prefix}-surface-dim: {surface_dim};
	--{prefix}-surface: {surface};
	--{prefix}-surface-tint: {surface_tint};
	--{prefix}-surface-bright: {surface_bright};
	--{prefix}-surface-container-lowest: {surface_container_lowest};
	--{prefix}-surface-container-low: {surface_container_low};
	--{prefix}-surface-container: {surface_container};
	--{prefix}-surface-container-high: {surface_container_high};
	--{prefix}-surface-container-highest: {surface_container_highest};
	--{prefix}-on-surface: {on_surface};
	--{prefix}-on-surface-variant: {on_surface_variant};
	--{prefix}-outline: {outline};
	--{prefix}-outline-variant: {outline_variant};
	--{prefix}-inverse-surface: {inverse_surface};
	--{prefix}-inverse-on-surface: {inverse_on_surface};
	--{prefix}-surface-variant: {surface_variant};
	--{prefix}-background: {background};
	--{prefix}-on-background: {on_background};
	--{prefix}-shadow: {shadow};
	--{prefix}-scrim: {scrim};
}}"#
		)
	}
}

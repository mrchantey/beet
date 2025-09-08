use material_colors::scheme::Scheme;
use material_colors::theme::Theme;




pub struct ThemeToCss {
	// pub prefix: String,
	/// The global class applied to the document in light mode
	/// This must be kept in sync with initColorScheme.js
	pub light_class: String,
	/// The global class applied to the document in dark mode
	/// This must be kept in sync with initColorScheme.js
	pub dark_class: String,
}

impl Default for ThemeToCss {
	fn default() -> Self {
		Self {
			// prefix: Self::DEFAULT_PREFIX.to_string(),
			light_class: Self::DEFAULT_LIGHT_CLASS.to_string(),
			dark_class: Self::DEFAULT_DARK_CLASS.to_string(),
		}
	}
}


impl ThemeToCss {
	// pub const DEFAULT_PREFIX: &'static str = "--bt-color";
	pub const DEFAULT_LIGHT_CLASS: &'static str = "scheme-light";
	pub const DEFAULT_DARK_CLASS: &'static str = "scheme-dark";

	pub fn map(&self, theme: &Theme) -> String {
		let Self {
			// prefix,
			light_class: light_scheme,
			dark_class: dark_scheme,
		} = self;

		let light = Self::scheme_to_css(light_scheme, &theme.schemes.light);
		let dark = Self::scheme_to_css(dark_scheme, &theme.schemes.dark);
		format!(
			r#"
		{light}
		{dark}
		:root{{
		/* a default that can be overridden, ie by a button state */
		--bt-opacity: 1;
		--bt-opacity-hover: 0.7;
		--bt-opacity-active: 0.5;
		--bt-opacity-disabled: 0.5;

		--bt-color-border: var(--bt-color-outline);
		--bt-color-text: var(--bt-color-on-surface);
		--bt-color-background: var(--bt-color-surface-container-lowest);
		--bt-color-border: var(--bt-color-outline);
		--bt-color-faint: var(--bt-color-outline);
		}}
		"#
		)
	}

	fn scheme_to_css(class: &str, scheme: &Scheme) -> String {
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
	--bt-color-primary: {primary};
	--bt-color-on-primary: {on_primary};
	--bt-color-primary-container: {primary_container};
	--bt-color-on-primary-container: {on_primary_container};
	--bt-color-inverse-primary: {inverse_primary};
	--bt-color-primary-fixed: {primary_fixed};
	--bt-color-primary-fixed-dim: {primary_fixed_dim};
	--bt-color-on-primary-fixed: {on_primary_fixed};
	--bt-color-on-primary-fixed-variant: {on_primary_fixed_variant};
	--bt-color-secondary: {secondary};
	--bt-color-on-secondary: {on_secondary};
	--bt-color-secondary-container: {secondary_container};
	--bt-color-on-secondary-container: {on_secondary_container};
	--bt-color-secondary-fixed: {secondary_fixed};
	--bt-color-secondary-fixed-dim: {secondary_fixed_dim};
	--bt-color-on-secondary-fixed: {on_secondary_fixed};
	--bt-color-on-secondary-fixed-variant: {on_secondary_fixed_variant};
	--bt-color-tertiary: {tertiary};
	--bt-color-on-tertiary: {on_tertiary};
	--bt-color-tertiary-container: {tertiary_container};
	--bt-color-on-tertiary-container: {on_tertiary_container};
	--bt-color-tertiary-fixed: {tertiary_fixed};
	--bt-color-tertiary-fixed-dim: {tertiary_fixed_dim};
	--bt-color-on-tertiary-fixed: {on_tertiary_fixed};
	--bt-color-on-tertiary-fixed-variant: {on_tertiary_fixed_variant};
	--bt-color-error: {error};
	--bt-color-on-error: {on_error};
	--bt-color-error-container: {error_container};
	--bt-color-on-error-container: {on_error_container};
	--bt-color-surface-dim: {surface_dim};
	--bt-color-surface: {surface};
	--bt-color-surface-tint: {surface_tint};
	--bt-color-surface-bright: {surface_bright};
	--bt-color-surface-container-lowest: {surface_container_lowest};
	--bt-color-surface-container-low: {surface_container_low};
	--bt-color-surface-container: {surface_container};
	--bt-color-surface-container-high: {surface_container_high};
	--bt-color-surface-container-highest: {surface_container_highest};
	--bt-color-on-surface: {on_surface};
	--bt-color-on-surface-variant: {on_surface_variant};
	--bt-color-outline: {outline};
	--bt-color-outline-variant: {outline_variant};
	--bt-color-inverse-surface: {inverse_surface};
	--bt-color-inverse-on-surface: {inverse_on_surface};
	--bt-color-surface-variant: {surface_variant};
	--bt-color-background: {background};
	--bt-color-on-background: {on_background};
	--bt-color-shadow: {shadow};
	--bt-color-scrim: {scrim};
}}"#
		)
	}
}
// avoid lightning dependencies
// #[cfg(test)]
// mod test {
// 	use crate::prelude::*;
// 	use lightningcss::stylesheet::ParserOptions;
// 	use lightningcss::stylesheet::StyleSheet;
// 	use material_colors::color::Argb;
// 	use material_colors::theme::ThemeBuilder;
// 	use sweet::prelude::*;

// 	#[test]
// 	fn works() {
// 		let css = ThemeToCss::default()
// 			.map(&ThemeBuilder::with_source(Argb::new(255, 255, 0, 0)).build());
// 		StyleSheet::parse(&css, ParserOptions::default()).xpect().to_be_ok();
// 	}
// }

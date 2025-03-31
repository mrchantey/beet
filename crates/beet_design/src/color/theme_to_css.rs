use material_colors::scheme::Scheme;
use material_colors::theme::Theme;




pub struct ThemeToCss {
	pub prefix: String,
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
			prefix: Self::DEFAULT_PREFIX.to_string(),
			light_class: Self::DEFAULT_LIGHT_CLASS.to_string(),
			dark_class: Self::DEFAULT_DARK_CLASS.to_string(),
		}
	}
}


impl ThemeToCss {
	pub const DEFAULT_PREFIX: &'static str = "--bt-color";
	pub const DEFAULT_LIGHT_CLASS: &'static str = "scheme-light";
	pub const DEFAULT_DARK_CLASS: &'static str = "scheme-dark";

	pub fn map(&self, theme: &Theme) -> String {
		let Self {
			prefix,
			light_class: light_scheme,
			dark_class: dark_scheme,
		} = self;

		let light =
			Self::scheme_to_css(light_scheme, prefix, &theme.schemes.light);
		let dark =
			Self::scheme_to_css(dark_scheme, prefix, &theme.schemes.dark);
		format!(
			r#"
		{light}
		{dark}
		:root{{
		/* a default that can be overridden, ie by a button state */
		{prefix}-opacity: 1;
		{prefix}-opacity-hover: 0.7;
		{prefix}-opacity-active: 0.5;
		{prefix}-opacity-disabled: 0.5;
		
		{prefix}-border: var({prefix}-outline);
		{prefix}-text: var({prefix}-on-surface);
		{prefix}-background: var({prefix}-surface);
		{prefix}-border: var({prefix}-outline);
		{prefix}-faint: var({prefix}-outline);
		}}
		"#
		)
	}

	fn scheme_to_css(class: &str, var_prefix: &str, scheme: &Scheme) -> String {
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
	{var_prefix}-primary: {primary};
	{var_prefix}-on-primary: {on_primary};
	{var_prefix}-primary-container: {primary_container};
	{var_prefix}-on-primary-container: {on_primary_container};
	{var_prefix}-inverse-primary: {inverse_primary};
	{var_prefix}-primary-fixed: {primary_fixed};
	{var_prefix}-primary-fixed-dim: {primary_fixed_dim};
	{var_prefix}-on-primary-fixed: {on_primary_fixed};
	{var_prefix}-on-primary-fixed-variant: {on_primary_fixed_variant};
	{var_prefix}-secondary: {secondary};
	{var_prefix}-on-secondary: {on_secondary};
	{var_prefix}-secondary-container: {secondary_container};
	{var_prefix}-on-secondary-container: {on_secondary_container};
	{var_prefix}-secondary-fixed: {secondary_fixed};
	{var_prefix}-secondary-fixed-dim: {secondary_fixed_dim};
	{var_prefix}-on-secondary-fixed: {on_secondary_fixed};
	{var_prefix}-on-secondary-fixed-variant: {on_secondary_fixed_variant};
	{var_prefix}-tertiary: {tertiary};
	{var_prefix}-on-tertiary: {on_tertiary};
	{var_prefix}-tertiary-container: {tertiary_container};
	{var_prefix}-on-tertiary-container: {on_tertiary_container};
	{var_prefix}-tertiary-fixed: {tertiary_fixed};
	{var_prefix}-tertiary-fixed-dim: {tertiary_fixed_dim};
	{var_prefix}-on-tertiary-fixed: {on_tertiary_fixed};
	{var_prefix}-on-tertiary-fixed-variant: {on_tertiary_fixed_variant};
	{var_prefix}-error: {error};
	{var_prefix}-on-error: {on_error};
	{var_prefix}-error-container: {error_container};
	{var_prefix}-on-error-container: {on_error_container};
	{var_prefix}-surface-dim: {surface_dim};
	{var_prefix}-surface: {surface};
	{var_prefix}-surface-tint: {surface_tint};
	{var_prefix}-surface-bright: {surface_bright};
	{var_prefix}-surface-container-lowest: {surface_container_lowest};
	{var_prefix}-surface-container-low: {surface_container_low};
	{var_prefix}-surface-container: {surface_container};
	{var_prefix}-surface-container-high: {surface_container_high};
	{var_prefix}-surface-container-highest: {surface_container_highest};
	{var_prefix}-on-surface: {on_surface};
	{var_prefix}-on-surface-variant: {on_surface_variant};
	{var_prefix}-outline: {outline};
	{var_prefix}-outline-variant: {outline_variant};
	{var_prefix}-inverse-surface: {inverse_surface};
	{var_prefix}-inverse-on-surface: {inverse_on_surface};
	{var_prefix}-surface-variant: {surface_variant};
	{var_prefix}-background: {background};
	{var_prefix}-on-background: {on_background};
	{var_prefix}-shadow: {shadow};
	{var_prefix}-scrim: {scrim};
}}"#
		)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use lightningcss::stylesheet::ParserOptions;
	use lightningcss::stylesheet::StyleSheet;
	use material_colors::color::Argb;
	use material_colors::theme::ThemeBuilder;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let css = ThemeToCss::default()
			.map(&ThemeBuilder::with_source(Argb::new(255, 255, 0, 0)).build());
		expect(StyleSheet::parse(&css, ParserOptions::default())).to_be_ok();

		// println!("{}", css);
	}
}

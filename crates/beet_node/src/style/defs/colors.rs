#![cfg_attr(rustfmt, rustfmt_skip)]
use beet_core::prelude::*;
use crate::style::*;
use crate::prelude::*;

token!(f32, OPACITY_HOVERED, "opacity-hovered");
token!(f32, OPACITY_FOCUSED, "opacity-focused");
token!(f32, OPACITY_PRESSED, "opacity-pressed");
token!(f32, OPACITY_DRAGGED, "opacity-dragged");

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



pub fn light_scheme() -> TokenMap {
	TokenMap::default()
		.with(colors::PRIMARY, tones::PRIMARY_40)
		.with(colors::ON_PRIMARY, tones::PRIMARY_100)
		.with(colors::PRIMARY_CONTAINER, tones::PRIMARY_90)
		.with(colors::ON_PRIMARY_CONTAINER, tones::PRIMARY_10)
		.with(colors::SECONDARY, tones::SECONDARY_40)
		.with(colors::ON_SECONDARY, tones::SECONDARY_100)
		.with(colors::SECONDARY_CONTAINER, tones::SECONDARY_90)
		.with(colors::ON_SECONDARY_CONTAINER, tones::SECONDARY_10)
		.with(colors::TERTIARY, tones::TERTIARY_40)
		.with(colors::ON_TERTIARY, tones::TERTIARY_100)
		.with(colors::TERTIARY_CONTAINER, tones::TERTIARY_90)
		.with(colors::ON_TERTIARY_CONTAINER, tones::TERTIARY_10)
		.with(colors::ERROR, tones::ERROR_40)
		.with(colors::ON_ERROR, tones::ERROR_100)
		.with(colors::ERROR_CONTAINER, tones::ERROR_90)
		.with(colors::ON_ERROR_CONTAINER, tones::ERROR_10)
		.with(colors::BACKGROUND, tones::NEUTRAL_99)
		.with(colors::ON_BACKGROUND, tones::NEUTRAL_10)
		.with(colors::SURFACE, tones::NEUTRAL_99)
		.with(colors::ON_SURFACE, tones::NEUTRAL_10)
		.with(colors::SURFACE_VARIANT, tones::NEUTRAL_VARIANT_90)
		.with(colors::ON_SURFACE_VARIANT, tones::NEUTRAL_VARIANT_30)
		.with(colors::OUTLINE, tones::NEUTRAL_VARIANT_50)
		.with(colors::OUTLINE_VARIANT, tones::NEUTRAL_VARIANT_80)
		.with(colors::SHADOW, tones::NEUTRAL_0)
		.with(colors::SCRIM, tones::NEUTRAL_0)
		.with(colors::INVERSE_SURFACE, tones::NEUTRAL_20)
		.with(colors::INVERSE_ON_SURFACE, tones::NEUTRAL_95)
		.with(colors::INVERSE_PRIMARY, tones::PRIMARY_80)
}

pub fn dark_scheme() -> TokenMap {
	TokenMap::default()
		.with(colors::PRIMARY, tones::PRIMARY_80)
		.with(colors::ON_PRIMARY, tones::PRIMARY_20)
		.with(colors::PRIMARY_CONTAINER, tones::PRIMARY_30)
		.with(colors::ON_PRIMARY_CONTAINER, tones::PRIMARY_90)
		.with(colors::SECONDARY, tones::SECONDARY_80)
		.with(colors::ON_SECONDARY, tones::SECONDARY_20)
		.with(colors::SECONDARY_CONTAINER, tones::SECONDARY_30)
		.with(colors::ON_SECONDARY_CONTAINER, tones::SECONDARY_90)
		.with(colors::TERTIARY, tones::TERTIARY_80)
		.with(colors::ON_TERTIARY, tones::TERTIARY_20)
		.with(colors::TERTIARY_CONTAINER, tones::TERTIARY_30)
		.with(colors::ON_TERTIARY_CONTAINER, tones::TERTIARY_90)
		.with(colors::ERROR, tones::ERROR_80)
		.with(colors::ON_ERROR, tones::ERROR_20)
		.with(colors::ERROR_CONTAINER, tones::ERROR_30)
		.with(colors::ON_ERROR_CONTAINER, tones::ERROR_80)
		.with(colors::BACKGROUND, tones::NEUTRAL_10)
		.with(colors::ON_BACKGROUND, tones::NEUTRAL_90)
		.with(colors::SURFACE, tones::NEUTRAL_10)
		.with(colors::ON_SURFACE, tones::NEUTRAL_90)
		.with(colors::SURFACE_VARIANT, tones::NEUTRAL_VARIANT_30)
		.with(colors::ON_SURFACE_VARIANT, tones::NEUTRAL_VARIANT_80)
		.with(colors::OUTLINE, tones::NEUTRAL_VARIANT_60)
		.with(colors::OUTLINE_VARIANT, tones::NEUTRAL_VARIANT_30)
		.with(colors::SHADOW, tones::NEUTRAL_0)
		.with(colors::SCRIM, tones::NEUTRAL_0)
		.with(colors::INVERSE_SURFACE, tones::NEUTRAL_90)
		.with(colors::INVERSE_ON_SURFACE, tones::NEUTRAL_20)
		.with(colors::INVERSE_PRIMARY, tones::PRIMARY_40)
}

#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use beet_core::prelude::*;
use material_colors::color::Argb;
use material_colors::theme::Palettes;
use material_colors::theme::ThemeBuilder;
use crate::style::material::colors;
use crate::style::material::tones;

/// Color conversion helpers for material-colors integration.
#[extend::ext(name=MaterialColorExt)]
pub impl Color {
	/// Converts to an ARGB representation with 8 bits per channel.
	fn to_argb(&self) -> Argb {
		let srgba = self.to_srgba_u8();
		Argb::new(srgba.alpha, srgba.red, srgba.green, srgba.blue)
	}
	fn from_argb(argb: Argb) -> Self {
		Color::srgba_u8(argb.red, argb.green, argb.blue, argb.alpha)
	}
}

/// Returns a [`Selector`] mapping semantic color tokens to their light-scheme tones.
pub fn light_scheme() -> Selector {
	Selector::new()
		.with_typed::<colors::Primary,                  tones::Primary40>()
		.with_typed::<colors::OnPrimary,                tones::Primary100>()
		.with_typed::<colors::PrimaryContainer,         tones::Primary90>()
		.with_typed::<colors::OnPrimaryContainer,       tones::Primary10>()
		.with_typed::<colors::InversePrimary,           tones::Primary80>()
		.with_typed::<colors::Secondary,                tones::Secondary40>()
		.with_typed::<colors::OnSecondary,              tones::Secondary100>()
		.with_typed::<colors::SecondaryContainer,       tones::Secondary90>()
		.with_typed::<colors::OnSecondaryContainer,     tones::Secondary10>()
		.with_typed::<colors::Tertiary,                 tones::Tertiary40>()
		.with_typed::<colors::OnTertiary,               tones::Tertiary100>()
		.with_typed::<colors::TertiaryContainer,        tones::Tertiary90>()
		.with_typed::<colors::OnTertiaryContainer,      tones::Tertiary10>()
		.with_typed::<colors::Error,                    tones::Error40>()
		.with_typed::<colors::OnError,                  tones::Error100>()
		.with_typed::<colors::ErrorContainer,           tones::Error90>()
		.with_typed::<colors::OnErrorContainer,         tones::Error10>()
		.with_typed::<colors::Background,               tones::Neutral99>()
		.with_typed::<colors::OnBackground,             tones::Neutral10>()
		.with_typed::<colors::Surface,                  tones::Neutral99>()
		.with_typed::<colors::OnSurface,                tones::Neutral10>()
		.with_typed::<colors::SurfaceVariant,           tones::NeutralVariant90>()
		.with_typed::<colors::OnSurfaceVariant,         tones::NeutralVariant30>()
		.with_typed::<colors::Outline,                  tones::NeutralVariant50>()
		.with_typed::<colors::OutlineVariant,           tones::NeutralVariant80>()
		.with_typed::<colors::Shadow,                   tones::Neutral0>()
		.with_typed::<colors::Scrim,                    tones::Neutral0>()
		.with_typed::<colors::InverseSurface,           tones::Neutral20>()
		.with_typed::<colors::InverseOnSurface,         tones::Neutral95>()
}

/// Returns a [`Selector`] mapping semantic color tokens to their dark-scheme tones.
pub fn dark_scheme() -> Selector {
	Selector::new()
		.with_typed::<colors::Primary,                  tones::Primary80>()
		.with_typed::<colors::OnPrimary,                tones::Primary20>()
		.with_typed::<colors::PrimaryContainer,         tones::Primary30>()
		.with_typed::<colors::OnPrimaryContainer,       tones::Primary90>()
		.with_typed::<colors::InversePrimary,           tones::Primary40>()
		.with_typed::<colors::Secondary,                tones::Secondary80>()
		.with_typed::<colors::OnSecondary,              tones::Secondary20>()
		.with_typed::<colors::SecondaryContainer,       tones::Secondary30>()
		.with_typed::<colors::OnSecondaryContainer,     tones::Secondary90>()
		.with_typed::<colors::Tertiary,                 tones::Tertiary80>()
		.with_typed::<colors::OnTertiary,               tones::Tertiary20>()
		.with_typed::<colors::TertiaryContainer,        tones::Tertiary30>()
		.with_typed::<colors::OnTertiaryContainer,      tones::Tertiary90>()
		.with_typed::<colors::Error,                    tones::Error80>()
		.with_typed::<colors::OnError,                  tones::Error20>()
		.with_typed::<colors::ErrorContainer,           tones::Error30>()
		.with_typed::<colors::OnErrorContainer,         tones::Error80>()
		.with_typed::<colors::Background,               tones::Neutral10>()
		.with_typed::<colors::OnBackground,             tones::Neutral90>()
		.with_typed::<colors::Surface,                  tones::Neutral10>()
		.with_typed::<colors::OnSurface,                tones::Neutral90>()
		.with_typed::<colors::SurfaceVariant,           tones::NeutralVariant30>()
		.with_typed::<colors::OnSurfaceVariant,         tones::NeutralVariant80>()
		.with_typed::<colors::Outline,                  tones::NeutralVariant60>()
		.with_typed::<colors::OutlineVariant,           tones::NeutralVariant30>()
		.with_typed::<colors::Shadow,                   tones::Neutral0>()
		.with_typed::<colors::Scrim,                    tones::Neutral0>()
		.with_typed::<colors::InverseSurface,           tones::Neutral90>()
		.with_typed::<colors::InverseOnSurface,         tones::Neutral20>()
}

/// Returns a [`Selector`] with concrete [`Color`] values for every palette tone,
/// generated from a seed color.
pub fn from_color(color: impl Into<Color>) -> Selector {
	let theme = ThemeBuilder::with_source(color.into().to_argb()).build();
	let Palettes { primary, secondary, tertiary, neutral, neutral_variant: nv, error } = theme.palettes;

	Selector::new()
		// ── Primary tones ─────────────────────────────────────────────────────
		.with_value::<tones::Primary0>(Color::from_argb(primary.tone(0))).unwrap()
		.with_value::<tones::Primary10>(Color::from_argb(primary.tone(10))).unwrap()
		.with_value::<tones::Primary20>(Color::from_argb(primary.tone(20))).unwrap()
		.with_value::<tones::Primary30>(Color::from_argb(primary.tone(30))).unwrap()
		.with_value::<tones::Primary40>(Color::from_argb(primary.tone(40))).unwrap()
		.with_value::<tones::Primary50>(Color::from_argb(primary.tone(50))).unwrap()
		.with_value::<tones::Primary60>(Color::from_argb(primary.tone(60))).unwrap()
		.with_value::<tones::Primary70>(Color::from_argb(primary.tone(70))).unwrap()
		.with_value::<tones::Primary80>(Color::from_argb(primary.tone(80))).unwrap()
		.with_value::<tones::Primary90>(Color::from_argb(primary.tone(90))).unwrap()
		.with_value::<tones::Primary95>(Color::from_argb(primary.tone(95))).unwrap()
		.with_value::<tones::Primary99>(Color::from_argb(primary.tone(99))).unwrap()
		.with_value::<tones::Primary100>(Color::from_argb(primary.tone(100))).unwrap()
		// ── Secondary tones ───────────────────────────────────────────────────
		.with_value::<tones::Secondary0>(Color::from_argb(secondary.tone(0))).unwrap()
		.with_value::<tones::Secondary10>(Color::from_argb(secondary.tone(10))).unwrap()
		.with_value::<tones::Secondary20>(Color::from_argb(secondary.tone(20))).unwrap()
		.with_value::<tones::Secondary30>(Color::from_argb(secondary.tone(30))).unwrap()
		.with_value::<tones::Secondary40>(Color::from_argb(secondary.tone(40))).unwrap()
		.with_value::<tones::Secondary50>(Color::from_argb(secondary.tone(50))).unwrap()
		.with_value::<tones::Secondary60>(Color::from_argb(secondary.tone(60))).unwrap()
		.with_value::<tones::Secondary70>(Color::from_argb(secondary.tone(70))).unwrap()
		.with_value::<tones::Secondary80>(Color::from_argb(secondary.tone(80))).unwrap()
		.with_value::<tones::Secondary90>(Color::from_argb(secondary.tone(90))).unwrap()
		.with_value::<tones::Secondary95>(Color::from_argb(secondary.tone(95))).unwrap()
		.with_value::<tones::Secondary99>(Color::from_argb(secondary.tone(99))).unwrap()
		.with_value::<tones::Secondary100>(Color::from_argb(secondary.tone(100))).unwrap()
		// ── Tertiary tones ────────────────────────────────────────────────────
		.with_value::<tones::Tertiary0>(Color::from_argb(tertiary.tone(0))).unwrap()
		.with_value::<tones::Tertiary10>(Color::from_argb(tertiary.tone(10))).unwrap()
		.with_value::<tones::Tertiary20>(Color::from_argb(tertiary.tone(20))).unwrap()
		.with_value::<tones::Tertiary30>(Color::from_argb(tertiary.tone(30))).unwrap()
		.with_value::<tones::Tertiary40>(Color::from_argb(tertiary.tone(40))).unwrap()
		.with_value::<tones::Tertiary50>(Color::from_argb(tertiary.tone(50))).unwrap()
		.with_value::<tones::Tertiary60>(Color::from_argb(tertiary.tone(60))).unwrap()
		.with_value::<tones::Tertiary70>(Color::from_argb(tertiary.tone(70))).unwrap()
		.with_value::<tones::Tertiary80>(Color::from_argb(tertiary.tone(80))).unwrap()
		.with_value::<tones::Tertiary90>(Color::from_argb(tertiary.tone(90))).unwrap()
		.with_value::<tones::Tertiary95>(Color::from_argb(tertiary.tone(95))).unwrap()
		.with_value::<tones::Tertiary99>(Color::from_argb(tertiary.tone(99))).unwrap()
		.with_value::<tones::Tertiary100>(Color::from_argb(tertiary.tone(100))).unwrap()
		// ── Neutral tones ─────────────────────────────────────────────────────
		.with_value::<tones::Neutral0>(Color::from_argb(neutral.tone(0))).unwrap()
		.with_value::<tones::Neutral10>(Color::from_argb(neutral.tone(10))).unwrap()
		.with_value::<tones::Neutral20>(Color::from_argb(neutral.tone(20))).unwrap()
		.with_value::<tones::Neutral30>(Color::from_argb(neutral.tone(30))).unwrap()
		.with_value::<tones::Neutral40>(Color::from_argb(neutral.tone(40))).unwrap()
		.with_value::<tones::Neutral50>(Color::from_argb(neutral.tone(50))).unwrap()
		.with_value::<tones::Neutral60>(Color::from_argb(neutral.tone(60))).unwrap()
		.with_value::<tones::Neutral70>(Color::from_argb(neutral.tone(70))).unwrap()
		.with_value::<tones::Neutral80>(Color::from_argb(neutral.tone(80))).unwrap()
		.with_value::<tones::Neutral90>(Color::from_argb(neutral.tone(90))).unwrap()
		.with_value::<tones::Neutral95>(Color::from_argb(neutral.tone(95))).unwrap()
		.with_value::<tones::Neutral99>(Color::from_argb(neutral.tone(99))).unwrap()
		.with_value::<tones::Neutral100>(Color::from_argb(neutral.tone(100))).unwrap()
		// ── NeutralVariant tones ──────────────────────────────────────────────
		.with_value::<tones::NeutralVariant0>(Color::from_argb(nv.tone(0))).unwrap()
		.with_value::<tones::NeutralVariant10>(Color::from_argb(nv.tone(10))).unwrap()
		.with_value::<tones::NeutralVariant20>(Color::from_argb(nv.tone(20))).unwrap()
		.with_value::<tones::NeutralVariant30>(Color::from_argb(nv.tone(30))).unwrap()
		.with_value::<tones::NeutralVariant40>(Color::from_argb(nv.tone(40))).unwrap()
		.with_value::<tones::NeutralVariant50>(Color::from_argb(nv.tone(50))).unwrap()
		.with_value::<tones::NeutralVariant60>(Color::from_argb(nv.tone(60))).unwrap()
		.with_value::<tones::NeutralVariant70>(Color::from_argb(nv.tone(70))).unwrap()
		.with_value::<tones::NeutralVariant80>(Color::from_argb(nv.tone(80))).unwrap()
		.with_value::<tones::NeutralVariant90>(Color::from_argb(nv.tone(90))).unwrap()
		.with_value::<tones::NeutralVariant95>(Color::from_argb(nv.tone(95))).unwrap()
		.with_value::<tones::NeutralVariant99>(Color::from_argb(nv.tone(99))).unwrap()
		.with_value::<tones::NeutralVariant100>(Color::from_argb(nv.tone(100))).unwrap()
		// ── Error tones ───────────────────────────────────────────────────────
		.with_value::<tones::Error0>(Color::from_argb(error.tone(0))).unwrap()
		.with_value::<tones::Error10>(Color::from_argb(error.tone(10))).unwrap()
		.with_value::<tones::Error20>(Color::from_argb(error.tone(20))).unwrap()
		.with_value::<tones::Error30>(Color::from_argb(error.tone(30))).unwrap()
		.with_value::<tones::Error40>(Color::from_argb(error.tone(40))).unwrap()
		.with_value::<tones::Error50>(Color::from_argb(error.tone(50))).unwrap()
		.with_value::<tones::Error60>(Color::from_argb(error.tone(60))).unwrap()
		.with_value::<tones::Error70>(Color::from_argb(error.tone(70))).unwrap()
		.with_value::<tones::Error80>(Color::from_argb(error.tone(80))).unwrap()
		.with_value::<tones::Error90>(Color::from_argb(error.tone(90))).unwrap()
		.with_value::<tones::Error95>(Color::from_argb(error.tone(95))).unwrap()
		.with_value::<tones::Error99>(Color::from_argb(error.tone(99))).unwrap()
		.with_value::<tones::Error100>(Color::from_argb(error.tone(100))).unwrap()
}

/// Returns a [`Selector`] with default opacity scalar values.
pub fn default_opacities() -> Selector {
	Selector::new()
		.with_value::<colors::OpacityHovered>(0.08_f32).unwrap()
		.with_value::<colors::OpacityFocused>(0.12_f32).unwrap()
		.with_value::<colors::OpacityPressed>(0.12_f32).unwrap()
		.with_value::<colors::OpacityDragged>(0.16_f32).unwrap()
}

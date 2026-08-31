#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use beet_core::prelude::*;
use material_colors::color::Argb;
use material_colors::palette::CorePalette;
use material_colors::palette::TonalPalette;
use crate::style::material::colors;
use crate::style::material::tones;
use crate::style::material::classes;

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

/// Returns a [`Rule`] mapping semantic color tokens to their light-scheme tones.
pub(crate) fn light_scheme() -> Rule {
	Rule::new()
		.with_selector(Selector::class(classes::LIGHT_SCHEME))
		.with_token(colors::Primary, tones::Primary40).unwrap()
		.with_token(colors::OnPrimary, tones::Primary100).unwrap()
		.with_token(colors::PrimaryContainer, tones::Primary90).unwrap()
		.with_token(colors::OnPrimaryContainer, tones::Primary10).unwrap()
		.with_token(colors::InversePrimary, tones::Primary80).unwrap()
		.with_token(colors::Secondary, tones::Secondary40).unwrap()
		.with_token(colors::OnSecondary, tones::Secondary100).unwrap()
		.with_token(colors::SecondaryContainer, tones::Secondary90).unwrap()
		.with_token(colors::OnSecondaryContainer, tones::Secondary10).unwrap()
		.with_token(colors::Tertiary, tones::Tertiary40).unwrap()
		.with_token(colors::OnTertiary, tones::Tertiary100).unwrap()
		.with_token(colors::TertiaryContainer, tones::Tertiary90).unwrap()
		.with_token(colors::OnTertiaryContainer, tones::Tertiary10).unwrap()
		.with_token(colors::Error, tones::Error40).unwrap()
		.with_token(colors::OnError, tones::Error100).unwrap()
		.with_token(colors::ErrorContainer, tones::Error90).unwrap()
		.with_token(colors::OnErrorContainer, tones::Error10).unwrap()
		.with_token(colors::Background, tones::NeutralLight94).unwrap()
		.with_token(colors::OnBackground, tones::NeutralLight10).unwrap()
		.with_token(colors::Surface, tones::NeutralLight94).unwrap()
		.with_token(colors::SurfaceDim, tones::NeutralLight90).unwrap()
		.with_token(colors::SurfaceBright, tones::NeutralLight99).unwrap()
		.with_token(colors::SurfaceTint, tones::Primary40).unwrap()
		.with_token(colors::SurfaceContainerLowest, tones::NeutralLight100).unwrap()
		.with_token(colors::SurfaceContainerLow, tones::NeutralLight95).unwrap()
		.with_token(colors::SurfaceContainer, tones::NeutralLight95).unwrap()
		.with_token(colors::SurfaceContainerHigh, tones::NeutralLight90).unwrap()
		.with_token(colors::SurfaceContainerHighest, tones::NeutralLight90).unwrap()
		.with_token(colors::OnSurface, tones::NeutralLight10).unwrap()
		.with_token(colors::SurfaceVariant, tones::NeutralVariant90).unwrap()
		.with_token(colors::OnSurfaceVariant, tones::NeutralVariant30).unwrap()
		.with_token(colors::Outline, tones::NeutralVariant50).unwrap()
		.with_token(colors::OutlineVariant, tones::NeutralVariant80).unwrap()
		.with_token(colors::Shadow, tones::NeutralLight0).unwrap()
		.with_token(colors::Scrim, tones::NeutralLight0).unwrap()
		.with_token(colors::InverseSurface, tones::NeutralDark20).unwrap()
		.with_token(colors::InverseOnSurface, tones::NeutralDark95).unwrap()
}

/// Returns a [`Rule`] mapping semantic color tokens to their dark-scheme tones.
pub(crate) fn dark_scheme() -> Rule {
	Rule::new()
		.with_selector(Selector::class(classes::DARK_SCHEME))
		.with_token(colors::Primary, tones::Primary80).unwrap()
		.with_token(colors::OnPrimary, tones::Primary20).unwrap()
		.with_token(colors::PrimaryContainer, tones::Primary30).unwrap()
		.with_token(colors::OnPrimaryContainer, tones::Primary90).unwrap()
		.with_token(colors::InversePrimary, tones::Primary40).unwrap()
		.with_token(colors::Secondary, tones::Secondary80).unwrap()
		.with_token(colors::OnSecondary, tones::Secondary20).unwrap()
		.with_token(colors::SecondaryContainer, tones::Secondary30).unwrap()
		.with_token(colors::OnSecondaryContainer, tones::Secondary90).unwrap()
		.with_token(colors::Tertiary, tones::Tertiary80).unwrap()
		.with_token(colors::OnTertiary, tones::Tertiary20).unwrap()
		.with_token(colors::TertiaryContainer, tones::Tertiary30).unwrap()
		.with_token(colors::OnTertiaryContainer, tones::Tertiary90).unwrap()
		.with_token(colors::Error, tones::Error80).unwrap()
		.with_token(colors::OnError, tones::Error20).unwrap()
		.with_token(colors::ErrorContainer, tones::Error30).unwrap()
		.with_token(colors::OnErrorContainer, tones::Error80).unwrap()
		.with_token(colors::Background, tones::NeutralDark8).unwrap()
		.with_token(colors::OnBackground, tones::NeutralDark90).unwrap()
		.with_token(colors::Surface, tones::NeutralDark8).unwrap()
		.with_token(colors::SurfaceDim, tones::NeutralDark0).unwrap()
		.with_token(colors::SurfaceBright, tones::NeutralDark30).unwrap()
		.with_token(colors::SurfaceTint, tones::Primary80).unwrap()
		.with_token(colors::SurfaceContainerLowest, tones::NeutralDark0).unwrap()
		.with_token(colors::SurfaceContainerLow, tones::NeutralDark10).unwrap()
		.with_token(colors::SurfaceContainer, tones::NeutralDark20).unwrap()
		.with_token(colors::SurfaceContainerHigh, tones::NeutralDark20).unwrap()
		.with_token(colors::SurfaceContainerHighest, tones::NeutralDark30).unwrap()
		.with_token(colors::OnSurface, tones::NeutralDark90).unwrap()
		.with_token(colors::SurfaceVariant, tones::NeutralVariant30).unwrap()
		.with_token(colors::OnSurfaceVariant, tones::NeutralVariant80).unwrap()
		.with_token(colors::Outline, tones::NeutralVariant60).unwrap()
		.with_token(colors::OutlineVariant, tones::NeutralVariant30).unwrap()
		.with_token(colors::Shadow, tones::NeutralDark0).unwrap()
		.with_token(colors::Scrim, tones::NeutralDark0).unwrap()
		.with_token(colors::InverseSurface, tones::NeutralLight90).unwrap()
		.with_token(colors::InverseOnSurface, tones::NeutralLight20).unwrap()
}

/// Returns color values for every palette tone in a [`Theme`].
///
/// Each key colour holds its own hue and chroma across its ramp, and an unset key
/// falls back to the seed-derived [`CorePalette`], so a bare `<Theme color=../>`
/// still generates exactly the Material palette the seed always produced. The
/// neutral key is split per mode: [`Theme::neutral_light`] seeds the light
/// scheme's surfaces and the dark scheme's inverse surfaces, and
/// [`Theme::neutral_dark`] seeds the reverse.
pub(crate) fn from_theme(theme: &Theme) -> Vec<(TokenKey, TokenValue)> {
	let core = CorePalette::of(theme.color.to_argb());
	// a key colour holds its own hue and chroma, rather than being pushed through
	// a Material variant, so a low-chroma key (the cream, the green-cast ink)
	// keeps the cast it was picked for.
	let key = |color: Option<Color>, seeded: TonalPalette| match color {
		Some(color) => TonalPalette::from_hct(color.to_argb().into()),
		None => seeded,
	};
	let primary = key(theme.primary, core.primary);
	let secondary = key(theme.secondary, core.secondary);
	let tertiary = key(theme.tertiary, core.tertiary);
	let neutral_light = key(theme.neutral_light, core.neutral);
	let neutral_dark = key(theme.neutral_dark, core.neutral);
	let nv = key(theme.neutral_variant, core.neutral_variant);
	let error = key(theme.error, core.error);

	Rule::new()
		// ── Primary tones ─────────────────────────────────────────────────────────────
		.with_value(tones::Primary0,Color::from_argb(primary.tone(0)))
		.with_value(tones::Primary10,Color::from_argb(primary.tone(10)))
		.with_value(tones::Primary20,Color::from_argb(primary.tone(20)))
		.with_value(tones::Primary30,Color::from_argb(primary.tone(30)))
		.with_value(tones::Primary40,Color::from_argb(primary.tone(40)))
		.with_value(tones::Primary50,Color::from_argb(primary.tone(50)))
		.with_value(tones::Primary60,Color::from_argb(primary.tone(60)))
		.with_value(tones::Primary70,Color::from_argb(primary.tone(70)))
		.with_value(tones::Primary80,Color::from_argb(primary.tone(80)))
		.with_value(tones::Primary90,Color::from_argb(primary.tone(90)))
		.with_value(tones::Primary95,Color::from_argb(primary.tone(95)))
		.with_value(tones::Primary99,Color::from_argb(primary.tone(99)))
		.with_value(tones::Primary100,Color::from_argb(primary.tone(100)))
		// ── Secondary tones ───────────────────────────────────────────────────────────
		.with_value(tones::Secondary0,Color::from_argb(secondary.tone(0)))
		.with_value(tones::Secondary10,Color::from_argb(secondary.tone(10)))
		.with_value(tones::Secondary20,Color::from_argb(secondary.tone(20)))
		.with_value(tones::Secondary30,Color::from_argb(secondary.tone(30)))
		.with_value(tones::Secondary40,Color::from_argb(secondary.tone(40)))
		.with_value(tones::Secondary50,Color::from_argb(secondary.tone(50)))
		.with_value(tones::Secondary60,Color::from_argb(secondary.tone(60)))
		.with_value(tones::Secondary70,Color::from_argb(secondary.tone(70)))
		.with_value(tones::Secondary80,Color::from_argb(secondary.tone(80)))
		.with_value(tones::Secondary90,Color::from_argb(secondary.tone(90)))
		.with_value(tones::Secondary95,Color::from_argb(secondary.tone(95)))
		.with_value(tones::Secondary99,Color::from_argb(secondary.tone(99)))
		.with_value(tones::Secondary100,Color::from_argb(secondary.tone(100)))
		// ── Tertiary tones ────────────────────────────────────────────────────────────
		.with_value(tones::Tertiary0,Color::from_argb(tertiary.tone(0)))
		.with_value(tones::Tertiary10,Color::from_argb(tertiary.tone(10)))
		.with_value(tones::Tertiary20,Color::from_argb(tertiary.tone(20)))
		.with_value(tones::Tertiary30,Color::from_argb(tertiary.tone(30)))
		.with_value(tones::Tertiary40,Color::from_argb(tertiary.tone(40)))
		.with_value(tones::Tertiary50,Color::from_argb(tertiary.tone(50)))
		.with_value(tones::Tertiary60,Color::from_argb(tertiary.tone(60)))
		.with_value(tones::Tertiary70,Color::from_argb(tertiary.tone(70)))
		.with_value(tones::Tertiary80,Color::from_argb(tertiary.tone(80)))
		.with_value(tones::Tertiary90,Color::from_argb(tertiary.tone(90)))
		.with_value(tones::Tertiary95,Color::from_argb(tertiary.tone(95)))
		.with_value(tones::Tertiary99,Color::from_argb(tertiary.tone(99)))
		.with_value(tones::Tertiary100,Color::from_argb(tertiary.tone(100)))
		// ── Neutral tones, light mode ─────────────────────────────────────────────────
		.with_value(tones::NeutralLight0,Color::from_argb(neutral_light.tone(0)))
		.with_value(tones::NeutralLight8,Color::from_argb(neutral_light.tone(8)))
		.with_value(tones::NeutralLight10,Color::from_argb(neutral_light.tone(10)))
		.with_value(tones::NeutralLight20,Color::from_argb(neutral_light.tone(20)))
		.with_value(tones::NeutralLight30,Color::from_argb(neutral_light.tone(30)))
		.with_value(tones::NeutralLight40,Color::from_argb(neutral_light.tone(40)))
		.with_value(tones::NeutralLight50,Color::from_argb(neutral_light.tone(50)))
		.with_value(tones::NeutralLight60,Color::from_argb(neutral_light.tone(60)))
		.with_value(tones::NeutralLight70,Color::from_argb(neutral_light.tone(70)))
		.with_value(tones::NeutralLight80,Color::from_argb(neutral_light.tone(80)))
		.with_value(tones::NeutralLight90,Color::from_argb(neutral_light.tone(90)))
		.with_value(tones::NeutralLight94,Color::from_argb(neutral_light.tone(94)))
		.with_value(tones::NeutralLight95,Color::from_argb(neutral_light.tone(95)))
		.with_value(tones::NeutralLight99,Color::from_argb(neutral_light.tone(99)))
		.with_value(tones::NeutralLight100,Color::from_argb(neutral_light.tone(100)))
		// ── Neutral tones, dark mode ──────────────────────────────────────────────────
		.with_value(tones::NeutralDark0,Color::from_argb(neutral_dark.tone(0)))
		.with_value(tones::NeutralDark8,Color::from_argb(neutral_dark.tone(8)))
		.with_value(tones::NeutralDark10,Color::from_argb(neutral_dark.tone(10)))
		.with_value(tones::NeutralDark20,Color::from_argb(neutral_dark.tone(20)))
		.with_value(tones::NeutralDark30,Color::from_argb(neutral_dark.tone(30)))
		.with_value(tones::NeutralDark40,Color::from_argb(neutral_dark.tone(40)))
		.with_value(tones::NeutralDark50,Color::from_argb(neutral_dark.tone(50)))
		.with_value(tones::NeutralDark60,Color::from_argb(neutral_dark.tone(60)))
		.with_value(tones::NeutralDark70,Color::from_argb(neutral_dark.tone(70)))
		.with_value(tones::NeutralDark80,Color::from_argb(neutral_dark.tone(80)))
		.with_value(tones::NeutralDark90,Color::from_argb(neutral_dark.tone(90)))
		.with_value(tones::NeutralDark94,Color::from_argb(neutral_dark.tone(94)))
		.with_value(tones::NeutralDark95,Color::from_argb(neutral_dark.tone(95)))
		.with_value(tones::NeutralDark99,Color::from_argb(neutral_dark.tone(99)))
		.with_value(tones::NeutralDark100,Color::from_argb(neutral_dark.tone(100)))
		// ── NeutralVariant tones ──────────────────────────────────────────────────────
		.with_value(tones::NeutralVariant0,Color::from_argb(nv.tone(0)))
		.with_value(tones::NeutralVariant10,Color::from_argb(nv.tone(10)))
		.with_value(tones::NeutralVariant20,Color::from_argb(nv.tone(20)))
		.with_value(tones::NeutralVariant30,Color::from_argb(nv.tone(30)))
		.with_value(tones::NeutralVariant40,Color::from_argb(nv.tone(40)))
		.with_value(tones::NeutralVariant50,Color::from_argb(nv.tone(50)))
		.with_value(tones::NeutralVariant60,Color::from_argb(nv.tone(60)))
		.with_value(tones::NeutralVariant70,Color::from_argb(nv.tone(70)))
		.with_value(tones::NeutralVariant80,Color::from_argb(nv.tone(80)))
		.with_value(tones::NeutralVariant90,Color::from_argb(nv.tone(90)))
		.with_value(tones::NeutralVariant95,Color::from_argb(nv.tone(95)))
		.with_value(tones::NeutralVariant99,Color::from_argb(nv.tone(99)))
		.with_value(tones::NeutralVariant100,Color::from_argb(nv.tone(100)))
		// ── Error tones ───────────────────────────────────────────────────────────────
		.with_value(tones::Error0,Color::from_argb(error.tone(0)))
		.with_value(tones::Error10,Color::from_argb(error.tone(10)))
		.with_value(tones::Error20,Color::from_argb(error.tone(20)))
		.with_value(tones::Error30,Color::from_argb(error.tone(30)))
		.with_value(tones::Error40,Color::from_argb(error.tone(40)))
		.with_value(tones::Error50,Color::from_argb(error.tone(50)))
		.with_value(tones::Error60,Color::from_argb(error.tone(60)))
		.with_value(tones::Error70,Color::from_argb(error.tone(70)))
		.with_value(tones::Error80,Color::from_argb(error.tone(80)))
		.with_value(tones::Error90,Color::from_argb(error.tone(90)))
		.with_value(tones::Error95,Color::from_argb(error.tone(95)))
		.with_value(tones::Error99,Color::from_argb(error.tone(99)))
		.with_value(tones::Error100,Color::from_argb(error.tone(100)))
		.into_iter().collect()
}

/// Returns default opacity scalar values.
pub(crate) fn default_opacities() -> Vec<(TokenKey, TokenValue)> {
	Rule::new()
		.with_value(colors::OpacityHovered,0.08_f32)
		.with_value(colors::OpacityFocused,0.12_f32)
		.with_value(colors::OpacityPressed,0.12_f32)
		.with_value(colors::OpacityDragged,0.16_f32)
		.into_iter().collect()
}

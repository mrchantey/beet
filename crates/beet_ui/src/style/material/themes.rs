#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use beet_core::prelude::*;
use material_colors::color::Argb;
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
		.with_extend(fixed_roles())
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
		.with_extend(fixed_roles())
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

/// The fixed accent roles, the same tones in both schemes: a fill that keeps
/// its colour when the scheme flips (a chip, a diagram's ramp), `Dim` its
/// stronger sibling, `OnFixed` the text on either and `OnFixedVariant` its
/// lower-emphasis variant.
fn fixed_roles() -> Rule {
	Rule::new()
		.with_token(colors::PrimaryFixed, tones::Primary90).unwrap()
		.with_token(colors::PrimaryFixedDim, tones::Primary80).unwrap()
		.with_token(colors::OnPrimaryFixed, tones::Primary10).unwrap()
		.with_token(colors::OnPrimaryFixedVariant, tones::Primary30).unwrap()
		.with_token(colors::SecondaryFixed, tones::Secondary90).unwrap()
		.with_token(colors::SecondaryFixedDim, tones::Secondary80).unwrap()
		.with_token(colors::OnSecondaryFixed, tones::Secondary10).unwrap()
		.with_token(colors::OnSecondaryFixedVariant, tones::Secondary30).unwrap()
		.with_token(colors::TertiaryFixed, tones::Tertiary90).unwrap()
		.with_token(colors::TertiaryFixedDim, tones::Tertiary80).unwrap()
		.with_token(colors::OnTertiaryFixed, tones::Tertiary10).unwrap()
		.with_token(colors::OnTertiaryFixedVariant, tones::Tertiary30).unwrap()
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

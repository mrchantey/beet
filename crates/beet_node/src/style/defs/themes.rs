use beet_core::prelude::*;
use material_colors::color::Argb;
use material_colors::theme::Palettes;
use material_colors::theme::ThemeBuilder;

use crate::style::TokenStore;
use crate::style::tones;


#[extend::ext(name=MaterialColorExt)]
pub impl Color {
	/// Converts the color to an sRGBA representation with 8 bits per channel.
	fn to_argb(&self) -> Argb {
		let srgba = self.to_srgba_u8();
		Argb::new(srgba.alpha, srgba.red, srgba.green, srgba.blue)
	}
	fn from_argb(argb: Argb) -> Self {
		Color::srgba_u8(argb.red, argb.green, argb.blue, argb.alpha)
	}
}


pub fn from_color(color: impl Into<Color>) -> TokenStore<Color> {
	let theme = ThemeBuilder::with_source(color.into().to_argb())
		// .variant(Variant::TonalSpot)
		// .color_match(false)
		.build();

	let Palettes {
		primary,
		secondary,
		tertiary,
		neutral,
		neutral_variant: nv,
		error,
	} = theme.palettes;

	TokenStore::<Color>::new()
		.with(tones::PRIMARY_0, Color::from_argb(primary.tone(0)))
		.with(tones::PRIMARY_10, Color::from_argb(primary.tone(10)))
		.with(tones::PRIMARY_20, Color::from_argb(primary.tone(20)))
		.with(tones::PRIMARY_30, Color::from_argb(primary.tone(30)))
		.with(tones::PRIMARY_40, Color::from_argb(primary.tone(40)))
		.with(tones::PRIMARY_50, Color::from_argb(primary.tone(50)))
		.with(tones::PRIMARY_60, Color::from_argb(primary.tone(60)))
		.with(tones::PRIMARY_70, Color::from_argb(primary.tone(70)))
		.with(tones::PRIMARY_80, Color::from_argb(primary.tone(80)))
		.with(tones::PRIMARY_90, Color::from_argb(primary.tone(90)))
		.with(tones::PRIMARY_95, Color::from_argb(primary.tone(95)))
		.with(tones::PRIMARY_99, Color::from_argb(primary.tone(99)))
		.with(tones::PRIMARY_100, Color::from_argb(primary.tone(100)))
		.with(tones::SECONDARY_0, Color::from_argb(secondary.tone(0)))
		.with(tones::SECONDARY_10, Color::from_argb(secondary.tone(10)))
		.with(tones::SECONDARY_20, Color::from_argb(secondary.tone(20)))
		.with(tones::SECONDARY_30, Color::from_argb(secondary.tone(30)))
		.with(tones::SECONDARY_40, Color::from_argb(secondary.tone(40)))
		.with(tones::SECONDARY_50, Color::from_argb(secondary.tone(50)))
		.with(tones::SECONDARY_60, Color::from_argb(secondary.tone(60)))
		.with(tones::SECONDARY_70, Color::from_argb(secondary.tone(70)))
		.with(tones::SECONDARY_80, Color::from_argb(secondary.tone(80)))
		.with(tones::SECONDARY_90, Color::from_argb(secondary.tone(90)))
		.with(tones::SECONDARY_95, Color::from_argb(secondary.tone(95)))
		.with(tones::SECONDARY_99, Color::from_argb(secondary.tone(99)))
		.with(tones::SECONDARY_100, Color::from_argb(secondary.tone(100)))
		.with(tones::TERTIARY_0, Color::from_argb(tertiary.tone(0)))
		.with(tones::TERTIARY_10, Color::from_argb(tertiary.tone(10)))
		.with(tones::TERTIARY_20, Color::from_argb(tertiary.tone(20)))
		.with(tones::TERTIARY_30, Color::from_argb(tertiary.tone(30)))
		.with(tones::TERTIARY_40, Color::from_argb(tertiary.tone(40)))
		.with(tones::TERTIARY_50, Color::from_argb(tertiary.tone(50)))
		.with(tones::TERTIARY_60, Color::from_argb(tertiary.tone(60)))
		.with(tones::TERTIARY_70, Color::from_argb(tertiary.tone(70)))
		.with(tones::TERTIARY_80, Color::from_argb(tertiary.tone(80)))
		.with(tones::TERTIARY_90, Color::from_argb(tertiary.tone(90)))
		.with(tones::TERTIARY_95, Color::from_argb(tertiary.tone(95)))
		.with(tones::TERTIARY_99, Color::from_argb(tertiary.tone(99)))
		.with(tones::TERTIARY_100, Color::from_argb(tertiary.tone(100)))
		.with(tones::NEUTRAL_0, Color::from_argb(neutral.tone(0)))
		.with(tones::NEUTRAL_10, Color::from_argb(neutral.tone(10)))
		.with(tones::NEUTRAL_20, Color::from_argb(neutral.tone(20)))
		.with(tones::NEUTRAL_30, Color::from_argb(neutral.tone(30)))
		.with(tones::NEUTRAL_40, Color::from_argb(neutral.tone(40)))
		.with(tones::NEUTRAL_50, Color::from_argb(neutral.tone(50)))
		.with(tones::NEUTRAL_60, Color::from_argb(neutral.tone(60)))
		.with(tones::NEUTRAL_70, Color::from_argb(neutral.tone(70)))
		.with(tones::NEUTRAL_80, Color::from_argb(neutral.tone(80)))
		.with(tones::NEUTRAL_90, Color::from_argb(neutral.tone(90)))
		.with(tones::NEUTRAL_95, Color::from_argb(neutral.tone(95)))
		.with(tones::NEUTRAL_99, Color::from_argb(neutral.tone(99)))
		.with(tones::NEUTRAL_100, Color::from_argb(neutral.tone(100)))
		.with(tones::NEUTRAL_VARIANT_0, Color::from_argb(nv.tone(0)))
		.with(tones::NEUTRAL_VARIANT_10, Color::from_argb(nv.tone(10)))
		.with(tones::NEUTRAL_VARIANT_20, Color::from_argb(nv.tone(20)))
		.with(tones::NEUTRAL_VARIANT_30, Color::from_argb(nv.tone(30)))
		.with(tones::NEUTRAL_VARIANT_40, Color::from_argb(nv.tone(40)))
		.with(tones::NEUTRAL_VARIANT_50, Color::from_argb(nv.tone(50)))
		.with(tones::NEUTRAL_VARIANT_60, Color::from_argb(nv.tone(60)))
		.with(tones::NEUTRAL_VARIANT_70, Color::from_argb(nv.tone(70)))
		.with(tones::NEUTRAL_VARIANT_80, Color::from_argb(nv.tone(80)))
		.with(tones::NEUTRAL_VARIANT_90, Color::from_argb(nv.tone(90)))
		.with(tones::NEUTRAL_VARIANT_95, Color::from_argb(nv.tone(95)))
		.with(tones::NEUTRAL_VARIANT_99, Color::from_argb(nv.tone(99)))
		.with(tones::NEUTRAL_VARIANT_100, Color::from_argb(nv.tone(100)))
		.with(tones::ERROR_0, Color::from_argb(error.tone(0)))
		.with(tones::ERROR_10, Color::from_argb(error.tone(10)))
		.with(tones::ERROR_20, Color::from_argb(error.tone(20)))
		.with(tones::ERROR_30, Color::from_argb(error.tone(30)))
		.with(tones::ERROR_40, Color::from_argb(error.tone(40)))
		.with(tones::ERROR_50, Color::from_argb(error.tone(50)))
		.with(tones::ERROR_60, Color::from_argb(error.tone(60)))
		.with(tones::ERROR_70, Color::from_argb(error.tone(70)))
		.with(tones::ERROR_80, Color::from_argb(error.tone(80)))
		.with(tones::ERROR_90, Color::from_argb(error.tone(90)))
		.with(tones::ERROR_95, Color::from_argb(error.tone(95)))
		.with(tones::ERROR_99, Color::from_argb(error.tone(99)))
		.with(tones::ERROR_100, Color::from_argb(error.tone(100)))
}

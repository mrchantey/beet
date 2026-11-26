//! The tonal palettes a [`Theme`] seeds, by name and tone, for consumers
//! outside the cascade: a script drawing in the brand colours, a page editing
//! them in the browser.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::material::themes::MaterialColorExt;
use crate::style::material::tones;
use beet_core::prelude::*;
use material_colors::palette::CorePalette;
use material_colors::palette::TonalPalette;
use serde::Serialize;

/// One Material palette a [`Theme`] key seeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemePalette {
	/// Seeded by [`Theme::primary`].
	Primary,
	/// Seeded by [`Theme::secondary`].
	Secondary,
	/// Seeded by [`Theme::tertiary`].
	Tertiary,
	/// Seeded by [`Theme::neutral_light`].
	NeutralLight,
	/// Seeded by [`Theme::neutral_dark`].
	NeutralDark,
	/// Seeded by [`Theme::neutral_variant`].
	NeutralVariant,
	/// Seeded by [`Theme::error`].
	Error,
}

impl ThemePalette {
	/// The palette's name, that of the [`Theme`] field keying it, ie
	/// `neutral_light`, so a script reads a stored theme and its tones by one
	/// vocabulary.
	pub fn name(&self) -> &'static str {
		match self {
			Self::Primary => "primary",
			Self::Secondary => "secondary",
			Self::Tertiary => "tertiary",
			Self::NeutralLight => "neutral_light",
			Self::NeutralDark => "neutral_dark",
			Self::NeutralVariant => "neutral_variant",
			Self::Error => "error",
		}
	}

	/// Every palette, in the order a [`Theme`] declares its keys.
	pub const ALL: [Self; 7] = [
		Self::Primary,
		Self::Secondary,
		Self::Tertiary,
		Self::NeutralLight,
		Self::NeutralDark,
		Self::NeutralVariant,
		Self::Error,
	];
}

/// The seven tonal palettes a [`Theme`] seeds. Each key colour holds its own
/// hue and chroma across its ramp, and an unset key falls back to the
/// seed-derived [`CorePalette`], so a bare `<Theme color=../>` generates exactly
/// the Material palette the seed always produced. The neutral key is split per
/// mode: [`Theme::neutral_light`] seeds the light scheme's surfaces and the dark
/// scheme's inverse surfaces, and [`Theme::neutral_dark`] the reverse.
pub struct ThemePalettes {
	primary: TonalPalette,
	secondary: TonalPalette,
	tertiary: TonalPalette,
	neutral_light: TonalPalette,
	neutral_dark: TonalPalette,
	neutral_variant: TonalPalette,
	error: TonalPalette,
}

impl ThemePalettes {
	/// The palettes `theme` seeds.
	pub fn new(theme: &Theme) -> Self {
		let core = CorePalette::of(theme.color.to_argb());
		// a key colour holds its own hue and chroma, rather than being pushed
		// through a Material variant, so a low-chroma key (the cream, a tinted
		// ink) keeps the cast it was picked for.
		let key = |color: Option<Color>, seeded: TonalPalette| match color {
			Some(color) => TonalPalette::from_hct(color.to_argb().into()),
			None => seeded,
		};
		Self {
			primary: key(theme.primary, core.primary),
			secondary: key(theme.secondary, core.secondary),
			tertiary: key(theme.tertiary, core.tertiary),
			neutral_light: key(theme.neutral_light, core.neutral),
			neutral_dark: key(theme.neutral_dark, core.neutral),
			neutral_variant: key(theme.neutral_variant, core.neutral_variant),
			error: key(theme.error, core.error),
		}
	}

	/// The colour of `palette` at `tone`, 0 black to 100 white.
	pub fn tone(&self, palette: ThemePalette, tone: u8) -> Color {
		Color::from_argb(self.palette(palette).tone(tone.min(100) as i32))
	}

	/// The key colour seeding `palette`: the theme's own where it sets one,
	/// else the one the seed derived.
	pub fn key(&self, palette: ThemePalette) -> Color {
		Color::from_argb(self.palette(palette).key_color().into())
	}

	fn palette(&self, palette: ThemePalette) -> &TonalPalette {
		match palette {
			ThemePalette::Primary => &self.primary,
			ThemePalette::Secondary => &self.secondary,
			ThemePalette::Tertiary => &self.tertiary,
			ThemePalette::NeutralLight => &self.neutral_light,
			ThemePalette::NeutralDark => &self.neutral_dark,
			ThemePalette::NeutralVariant => &self.neutral_variant,
			ThemePalette::Error => &self.error,
		}
	}

	/// Every tone of every palette as hex, the form a script reads.
	pub fn tones(&self) -> ThemeTones {
		ThemeTones(
			ThemePalette::ALL
				.into_iter()
				.map(|palette| {
					let ramp = (0..=100)
						.map(|tone| SmolStr::from(self.tone(palette, tone).to_srgba().to_hex()))
						.collect();
					(palette, ramp)
				})
				.collect(),
		)
	}

	/// Every tone token the cascade declares, with the palette and tone it
	/// holds: the `:root` tones `from_theme` bakes.
	pub fn tokens() -> Vec<(ThemePalette, u8, Token)> {
		use ThemePalette::*;
		macro_rules! ramp {
			($($palette:ident: $($token:ident = $tone:literal),+;)+) => {
				vec![$($(($palette, $tone, tones::$token.into())),+),+]
			};
		}
		ramp! {
			Primary: Primary0 = 0, Primary10 = 10, Primary20 = 20, Primary30 = 30, Primary40 = 40, Primary50 = 50, Primary60 = 60, Primary70 = 70, Primary80 = 80, Primary90 = 90, Primary95 = 95, Primary99 = 99, Primary100 = 100;
			Secondary: Secondary0 = 0, Secondary10 = 10, Secondary20 = 20, Secondary30 = 30, Secondary40 = 40, Secondary50 = 50, Secondary60 = 60, Secondary70 = 70, Secondary80 = 80, Secondary90 = 90, Secondary95 = 95, Secondary99 = 99, Secondary100 = 100;
			Tertiary: Tertiary0 = 0, Tertiary10 = 10, Tertiary20 = 20, Tertiary30 = 30, Tertiary40 = 40, Tertiary50 = 50, Tertiary60 = 60, Tertiary70 = 70, Tertiary80 = 80, Tertiary90 = 90, Tertiary95 = 95, Tertiary99 = 99, Tertiary100 = 100;
			NeutralLight: NeutralLight0 = 0, NeutralLight8 = 8, NeutralLight10 = 10, NeutralLight20 = 20, NeutralLight30 = 30, NeutralLight40 = 40, NeutralLight50 = 50, NeutralLight60 = 60, NeutralLight70 = 70, NeutralLight80 = 80, NeutralLight90 = 90, NeutralLight94 = 94, NeutralLight95 = 95, NeutralLight99 = 99, NeutralLight100 = 100;
			NeutralDark: NeutralDark0 = 0, NeutralDark8 = 8, NeutralDark10 = 10, NeutralDark20 = 20, NeutralDark30 = 30, NeutralDark40 = 40, NeutralDark50 = 50, NeutralDark60 = 60, NeutralDark70 = 70, NeutralDark80 = 80, NeutralDark90 = 90, NeutralDark94 = 94, NeutralDark95 = 95, NeutralDark99 = 99, NeutralDark100 = 100;
			NeutralVariant: NeutralVariant0 = 0, NeutralVariant10 = 10, NeutralVariant20 = 20, NeutralVariant30 = 30, NeutralVariant40 = 40, NeutralVariant50 = 50, NeutralVariant60 = 60, NeutralVariant70 = 70, NeutralVariant80 = 80, NeutralVariant90 = 90, NeutralVariant95 = 95, NeutralVariant99 = 99, NeutralVariant100 = 100;
			Error: Error0 = 0, Error10 = 10, Error20 = 20, Error30 = 30, Error40 = 40, Error50 = 50, Error60 = 60, Error70 = 70, Error80 = 80, Error90 = 90, Error95 = 95, Error99 = 99, Error100 = 100;
		}
	}
}

/// Every tone 0 to 100 of each [`ThemePalette`] as hex, serialized as an
/// object keyed by [`ThemePalette::name`] in [`ThemePalette::ALL`] order, so a script
/// reads `tones.neutral_light[94]`. The same theme serializes to the same
/// bytes, so a derived file may name it as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTones(pub Vec<(ThemePalette, Vec<SmolStr>)>);

impl Serialize for ThemeTones {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		use serde::ser::SerializeMap;
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (palette, ramp) in &self.0 {
			map.serialize_entry(palette.name(), ramp)?;
		}
		map.end()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::material::*;
	use beet_core::prelude::*;

	/// The site's brand keys sit at their palettes' anchor tones, which a
	/// script drawing in them relies on.
	#[beet_core::test]
	fn reads_a_tone() {
		let theme = Theme {
			primary: Some(Color::srgb_u8(0x00, 0x6c, 0x4f)),
			neutral_light: Some(Color::srgb_u8(0xf4, 0xee, 0xde)),
			..default()
		};
		let tones = serde_json::to_value(ThemePalettes::new(&theme).tones()).unwrap();
		tones["primary"][40].as_str().unwrap().xpect_eq("#006C4F");
		tones["neutral_light"][94].as_str().unwrap().xpect_eq("#F4EEDE");
		tones["error"].as_array().unwrap().len().xpect_eq(101);
	}
}

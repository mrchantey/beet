//! [`ThemeTones`]: every tone of the palettes the active [`Theme`] seeds, the
//! one place they are computed.
//!
//! The colour maths runs once per theme, as the theme lands, into the
//! [`ThemeTones`] resource, and everything drawing in the brand colours reads
//! it rather than recomputing: the `:root` tone declarations the css bake and
//! the charcell cascade resolve, a role resolved outside the cascade
//! ([`ThemeTones::resolve`]: a browser tint, a social card, a syndicated
//! record), a page's scripts (`<ThemeData/>`) and a derived file's script (a
//! logo drawn in the brand), which names the resource as its input.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::material::themes;
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

/// Every tone 0 to 100 of the seven tonal palettes the active [`Theme`] seeds,
/// with each palette's key colour, kept current by [`MaterialStylePlugin`] as
/// the theme changes.
///
/// Each key colour holds its own hue and chroma across its ramp, and an unset
/// key falls back to the seed-derived [`CorePalette`], so a bare
/// `<Theme color=../>` generates exactly the Material palette the seed always
/// produced. The neutral key is split per mode: [`Theme::neutral_light`] seeds
/// the light scheme's surfaces and the dark scheme's inverse surfaces, and
/// [`Theme::neutral_dark`] the reverse.
///
/// Serializes as an object of hex ramps keyed by [`ThemePalette::name`] in
/// [`ThemePalette::ALL`] order, so a script reads `tones.neutral_light[94]`,
/// and the same theme serializes to the same bytes, so a derived file may name
/// the resource as an input.
#[derive(Debug, Clone, PartialEq, Resource, Reflect)]
#[reflect(opaque)]
#[reflect(Resource, Default, Serialize)]
// a short stable path, since a derived file names its inputs by path
#[type_path = "beet_ui"]
pub struct ThemeTones(Vec<(ThemePalette, ThemeRamp)>);

/// One palette's key colour and its every tone.
#[derive(Debug, Clone, PartialEq)]
struct ThemeRamp {
	key: Color,
	/// Tones 0 to 100, black to white.
	tones: Vec<Color>,
}

impl Default for ThemeTones {
	fn default() -> Self { Self::new(&Theme::default()) }
}

impl ThemeTones {
	/// The tones `theme` seeds.
	pub fn new(theme: &Theme) -> Self {
		let core = CorePalette::of(theme.color.to_argb());
		// a key colour holds its own hue and chroma, rather than being pushed
		// through a Material variant, so a low-chroma key (the cream, a tinted
		// ink) keeps the cast it was picked for.
		let seeded = |palette: ThemePalette| -> (Option<Color>, &TonalPalette) {
			match palette {
				ThemePalette::Primary => (theme.primary, &core.primary),
				ThemePalette::Secondary => (theme.secondary, &core.secondary),
				ThemePalette::Tertiary => (theme.tertiary, &core.tertiary),
				ThemePalette::NeutralLight => (theme.neutral_light, &core.neutral),
				ThemePalette::NeutralDark => (theme.neutral_dark, &core.neutral),
				ThemePalette::NeutralVariant => (theme.neutral_variant, &core.neutral_variant),
				ThemePalette::Error => (theme.error, &core.error),
			}
		};
		Self(
			ThemePalette::ALL
				.into_iter()
				.map(|palette| {
					let ramp = match seeded(palette) {
						(Some(key), _) => TonalPalette::from_hct(key.to_argb().into()),
						(None, seeded) => seeded.clone(),
					};
					(palette, ThemeRamp {
						key: Color::from_argb(ramp.key_color().into()),
						tones: (0..=100).map(|tone| Color::from_argb(ramp.tone(tone))).collect(),
					})
				})
				.collect(),
		)
	}

	/// The colour of `palette` at `tone`, 0 black to 100 white.
	pub fn tone(&self, palette: ThemePalette, tone: u8) -> Color {
		self.ramp(palette).tones[tone.min(100) as usize]
	}

	/// The key colour seeding `palette`: the theme's own where it sets one,
	/// else the one the seed derived.
	pub fn key(&self, palette: ThemePalette) -> Color { self.ramp(palette).key }

	fn ramp(&self, palette: ThemePalette) -> &ThemeRamp {
		self.0
			.iter()
			.find(|(held, _)| *held == palette)
			.map(|(_, ramp)| ramp)
			.expect("tones hold every palette")
	}

	/// The colour `role` takes in `scheme`, ie [`colors::Background`] of the
	/// light scheme: the scheme's tone for that role, read exactly as the
	/// cascade reads it.
	///
	/// For a consumer outside the cascade, ie a syndicated record naming the
	/// site's colours, which has no element to resolve against.
	///
	/// # Errors
	/// Errors when `role` is not a colour role of the scheme.
	///
	/// [`colors::Background`]: crate::style::material::colors::Background
	pub fn resolve(&self, scheme: ColorScheme, role: impl Into<Token>) -> Result<Color> {
		let roles = match scheme {
			ColorScheme::Light => themes::light_scheme(),
			ColorScheme::Dark => themes::dark_scheme(),
		};
		match roles.get(&role.into())? {
			TokenValue::Token(tone) => Rule::new()
				.with_extend(self.declarations())
				.get_typed::<Color>(tone),
			TokenValue::Value(value) => value.into_typed::<Color>(),
		}
	}

	/// The `:root` tone declarations: every token of [`Self::tokens`] at its
	/// palette's tone.
	pub(crate) fn declarations(&self) -> Vec<(TokenKey, TokenValue)> {
		Self::tokens()
			.into_iter()
			.fold(Rule::new(), |rule, (palette, tone, token)| {
				rule.with_value_untyped(token, self.tone(palette, tone))
					.expect("a colour serializes")
			})
			.into_iter()
			.collect()
	}

	/// Every tone token the cascade declares, with the palette and tone it
	/// holds.
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

impl Serialize for ThemeTones {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		use serde::ser::SerializeMap;
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (palette, ramp) in &self.0 {
			let hex = ramp.tones.iter().map(|tone| tone.to_srgba().to_hex()).collect::<Vec<_>>();
			map.serialize_entry(palette.name(), &hex)?;
		}
		map.end()
	}
}

#[cfg(test)]
mod test {
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
		let tones = Value::from_serde(ThemeTones::new(&theme)).unwrap();
		tones["primary"][40].as_str().unwrap().xpect_eq("#006C4F");
		tones["neutral_light"][94].as_str().unwrap().xpect_eq("#F4EEDE");
		tones["error"].as_list().unwrap().len().xpect_eq(101);
	}
}

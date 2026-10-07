//! `site.standard.theme.basic` and `site.standard.theme.color`, the colours a
//! publication asks a foreign reader to render it in.
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// `site.standard.theme.basic`: four colours a reader renders a publication's
/// documents in, named by what each one colours.
///
/// Written with its `$type`, as Leaflet writes it, and each colour as the
/// member of a union.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[serde(tag = "$type", rename = "site.standard.theme.basic")]
#[reflect(Default, Serialize, Deserialize)]
pub struct ThemeBasic {
	/// The content background.
	pub background: ThemeColorRgb,
	/// The content text.
	pub foreground: ThemeColorRgb,
	/// Links and button backgrounds.
	pub accent: ThemeColorRgb,
	/// Text on a button.
	#[serde(rename = "accentForeground")]
	pub accent_foreground: ThemeColorRgb,
}

impl ThemeBasic {
	/// `theme`'s light scheme, the four roles that colour exactly what the
	/// lexicon names: [`Background`] the content background, [`OnBackground`]
	/// its text, [`Primary`] links and buttons, [`OnPrimary`] the text on them.
	///
	/// The resolved tones rather than the [`Theme`] keys, since `OnPrimary` is
	/// contrast the palette derives and no brand declares. The light scheme
	/// because a foreign reader renders the record on a surface of its own and
	/// cannot know which scheme a visitor here would see.
	///
	/// [`Background`]: material::colors::Background
	/// [`OnBackground`]: material::colors::OnBackground
	/// [`Primary`]: material::colors::Primary
	/// [`OnPrimary`]: material::colors::OnPrimary
	pub fn from_theme(theme: &Theme) -> Result<Self> {
		let color = |role: Token| {
			theme
				.resolve(ColorScheme::Light, role)
				.map(ThemeColorRgb::from)
		};
		Self {
			background: color(material::colors::Background.into())?,
			foreground: color(material::colors::OnBackground.into())?,
			accent: color(material::colors::Primary.into())?,
			accent_foreground: color(material::colors::OnPrimary.into())?,
		}
		.xok()
	}
}

/// `site.standard.theme.color#rgb`: an opaque colour, each channel 0 to 255.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[serde(tag = "$type", rename = "site.standard.theme.color#rgb")]
#[reflect(Default, Serialize, Deserialize)]
pub struct ThemeColorRgb {
	/// The red channel.
	pub r: u8,
	/// The green channel.
	pub g: u8,
	/// The blue channel.
	pub b: u8,
}

/// The colour's sRGB channels, its alpha dropped: the lexicon's `rgb` is
/// opaque.
impl From<Color> for ThemeColorRgb {
	fn from(color: Color) -> Self {
		let [r, g, b, _] = color.to_srgba().to_u8_array();
		Self { r, g, b }
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_ui::prelude::*;

	/// The four roles are the light scheme's: a light background under dark
	/// text, and the primary under its own contrast tone.
	#[beet_core::test]
	fn resolves_the_light_scheme() {
		let theme = Theme::default();
		let basic = ThemeBasic::from_theme(&theme).unwrap();
		let light = |role: Token| {
			ThemeColorRgb::from(
				theme.resolve(ColorScheme::Light, role).unwrap(),
			)
		};
		basic
			.accent_foreground
			.xpect_eq(light(material::colors::OnPrimary.into()));
		(basic.background.g > 200 && basic.foreground.g < 60).xpect_true();
	}

	/// The wire form Leaflet writes, every colour a union member.
	#[beet_core::test]
	fn round_trips_the_wire_form() {
		let json = r#"{"$type":"site.standard.theme.basic","background":{"$type":"site.standard.theme.color#rgb","r":255,"g":255,"b":255},"foreground":{"$type":"site.standard.theme.color#rgb","r":0,"g":0,"b":0},"accent":{"$type":"site.standard.theme.color#rgb","r":116,"g":145,"b":0},"accentForeground":{"$type":"site.standard.theme.color#rgb","r":255,"g":255,"b":255}}"#;
		let theme = serde_json::from_str::<ThemeBasic>(json).unwrap();
		theme.accent.xpect_eq(ThemeColorRgb {
			r: 116,
			g: 145,
			b: 0,
		});
		serde_json::to_string(&theme).unwrap().xpect_eq(json);
	}
}

//! `ThemeData` widget: the active theme as json a page's scripts read.
use crate::prelude::*;
use crate::style::CssVariable;
use crate::style::StyleQuery;
use crate::widgets::BakedStylesheet;
use beet_core::prelude::*;

/// Emits the active [`ThemeTones`] as `<script type="application/json"
/// id="beet-theme">`, for a page whose scripts draw in or edit the brand
/// colours: a logo designer, a palette editor.
///
/// The object holds three maps, each keyed by [`ThemePalette::name`]:
/// - `keys`: each palette's key colour, the theme's own where it sets one.
/// - `tones`: every tone 0 to 100 as hex, [`ThemeTones`].
/// - `variables`: the css variable each tone the cascade declares is named by
///   in the page's [`Stylesheet`], keyed by tone, so a script overriding a tone
///   on `<html>` recolours every role reading it, whatever the sheet's
///   [`FormatVariables`](crate::style::FormatVariables) chose.
///
/// Only a page asking for it carries it: the tones are a few kilobytes.
#[template(system)]
pub fn ThemeData(
	tones: Res<ThemeTones>,
	style: StyleQuery,
) -> Result<impl Bundle> {
	let names = style.variable_names(&BakedStylesheet::builder(false))?;
	let tokens = ThemeTones::tokens();
	let mut keys = Map::default();
	let mut variables = Map::default();
	for palette in ThemePalette::ALL {
		keys.insert(palette.name(), tones.key(palette).to_srgba().to_hex());
		let ramp = tokens
			.iter()
			.filter(|(held, ..)| *held == palette)
			.filter_map(|(_, tone, token)| {
				names.get(&CssVariable::from_token_key(token.key())).map(
					|name| {
						(tone.to_string().into(), Value::str(name.to_string()))
					},
				)
			})
			.collect::<Map>();
		variables.insert(palette.name(), ramp);
	}
	let json = value!({
		"keys": keys,
		"tones": Value::from_serde(&*tones)?,
		"variables": variables,
	})
	.to_json_string();
	rsx! {
		<script type="application/json" id="beet-theme">{json}</script>
	}
	.xok()
}

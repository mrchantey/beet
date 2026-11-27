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
pub fn ThemeData(tones: Res<ThemeTones>, style: StyleQuery) -> Result<impl Bundle> {
	use serde_json::Map;
	use serde_json::Value as Json;
	let names = style.variable_names(&BakedStylesheet::builder(false))?;
	let keys = ThemePalette::ALL
		.into_iter()
		.map(|palette| {
			let hex = tones.key(palette).to_srgba().to_hex();
			(palette.name().to_string(), Json::from(hex))
		})
		.collect::<Map<_, _>>();
	let mut variables = ThemePalette::ALL
		.into_iter()
		.map(|palette| (palette.name().to_string(), Json::Object(Map::new())))
		.collect::<Map<_, _>>();
	for (palette, tone, token) in ThemeTones::tokens() {
		if let Some(name) = names.get(&CssVariable::from_token_key(token.key()))
			&& let Some(Json::Object(ramp)) = variables.get_mut(palette.name())
		{
			ramp.insert(tone.to_string(), Json::from(name.to_string()));
		}
	}
	let json = serde_json::to_string(&serde_json::json!({
		"keys": keys,
		"tones": &*tones,
		"variables": variables,
	}))?;
	rsx! {
		<script type="application/json" id="beet-theme">{json}</script>
	}
	.xok()
}

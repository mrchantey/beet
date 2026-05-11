use crate::style::material::*;
use crate::style::*;
use crate::token::TokenStore;
use beet_core::prelude::*;
pub struct MaterialStylePlugin {
	color: Color,
}

impl MaterialStylePlugin {
	pub fn new(color: impl Into<Color>) -> Self {
		Self {
			color: color.into(),
		}
	}
}

impl Default for MaterialStylePlugin {
	fn default() -> Self {
		Self {
			color: palettes::basic::GREEN.into(),
		}
	}
}

impl Plugin for MaterialStylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<CssPlugin>()
			.insert_resource(default_token_store(self.color.clone()));
		app.world_mut()
			.get_resource_or_init::<CssTokenMap>()
			.extend(default_token_map());
	}
}




pub fn default_token_map() -> CssTokenMap {
	CssTokenMap::default()
		.with_extend(tones::token_map())
		.with_extend(colors::token_map())
		.with_extend(geometry::token_map())
		.with_extend(motion::token_map())
		.with_extend(typography::token_map())
}

/// All default material declarations and classes
pub fn default_token_store(color: impl Into<Color>) -> TokenStore {
	TokenStore::default()
		.with_extend(default_declarations(color))
		.with_extend(rules::all_rules())
		.with_value(themes::LightScheme, themes::light_scheme())
		.unwrap()
		.with_value(themes::DarkScheme, themes::dark_scheme())
		.unwrap()
}

/// Returns a [`Rule`] with all material design default values.
pub fn default_declarations(color: impl Into<Color>) -> TokenStore {
	TokenStore::default()
		.with_extend(themes::from_color(color))
		.with_extend(themes::default_opacities())
		.with_extend(typography::default_typography())
		.with_extend(geometry::default_shapes())
		.with_extend(geometry::default_elevations())
		.with_extend(motion::default_durations())
		.with_extend(motion::default_motions())
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn material_token_store() {
		MaterialStylePlugin::world()
			.spawn_empty()
			.with_state::<StyleQuery, _>(|entity, query| {
				query
					.collect_token_store(entity)
					.into_iter()
					.collect::<HashMap<_, _>>()
					.xmap(|map| serde_json::to_string_pretty(&map).unwrap())
			})
			.xpect_contains(r#""key": "rust:io.crates/beet_ui/style/material/colors/OnPrimary""#);
	}
	#[test]
	fn material_css() {
		MaterialStylePlugin::world()
			.spawn(rsx! {
				<div class="text-primary">hello world!</div>
			})
			.with_state::<StyleQuery, _>(|entity, query| {
				query.build_css(&default(), entity)
			})
			.xunwrap()
			.xpect_contains("--io-crates-beet-ui-style-material-motion-short2: 100ms;")
			.xpect_contains("--io-crates-beet-ui-style-material-typography-headline-large-weight: var(--io-crates-beet-ui-style-material-typography-weight-regular);");
	}
}

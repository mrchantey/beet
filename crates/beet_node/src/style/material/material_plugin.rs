use crate::style::material::*;
use crate::style::*;
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
			.insert_resource(default_rule_store(self.color.clone()));
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
	// .merge(typography::token_map())
}

/// All default material declarations and classes
pub fn default_rule_store(color: impl Into<Color>) -> RuleStore {
	RuleStore::default()
		.with(default_declarations(color))
		.with(themes::light_scheme())
		.with(themes::dark_scheme())
		.with(rules::hero_heading())
}

/// Returns a [`Rule`] with all material design default values.
pub fn default_declarations(color: impl Into<Color>) -> Rule {
	Rule::default()
		.extend(themes::from_color(color))
		.extend(themes::default_opacities())
		.extend(typography::default_typography())
		.extend(geometry::default_shapes())
		.extend(geometry::default_elevations())
		.extend(motion::default_durations())
		.extend(motion::default_motions())
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn validate_rules() {
		let mut world = MaterialStylePlugin::world();
		let css = world
			.spawn(rsx! {
				<div class="text-primary">hello world!</div>
			})
			.with_state::<StyleQuery, _>(|entity, query| {
				query.build_css(&default(), entity)
			})
			.xunwrap();
		println!("Generated CSS: \n{css}");
	}
}

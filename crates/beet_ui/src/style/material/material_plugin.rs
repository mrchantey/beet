use crate::prelude::*;
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
			.insert_resource(default_rule_set(self.color.clone()));
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

/// All default material declarations and component rules.
pub fn default_rule_set(color: impl Into<Color>) -> RuleSet {
	RuleSet::new(default_declarations(color))
		.with_rules(rules::all_rules())
		.with_rule(themes::light_scheme())
		.with_rule(themes::dark_scheme())
}

/// Returns a [`Rule`] with all material design default values.
///
/// Includes light-scheme semantic colors as the default, overrideable
/// with `.dark-scheme` class.
pub fn default_declarations(color: impl Into<Color>) -> Rule {
	Rule::new()
		.extend_declarations(themes::from_color(color))
		.extend_declarations(themes::light_scheme())
		.extend_declarations(themes::default_opacities())
		.extend_declarations(typography::default_typography())
		.extend_declarations(geometry::default_shapes())
		.extend_declarations(geometry::default_elevations())
		.extend_declarations(motion::default_durations())
		.extend_declarations(motion::default_motions())
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn material_rule_set() {
		MaterialStylePlugin::world()
			.spawn_empty()
			.with_state::<StyleQuery, _>(|_entity, query| {
				// OnPrimary is in the light/dark scheme rules, not the root default rule
				query
					.rule_set()
					.as_ref()
					.unwrap()
					.rules()
					.any(|rule| rule.get(&colors::OnPrimary.into()).is_ok())
			})
			.xpect_true();
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
			.xpect_contains(
				"--io-crates-beet-ui-style-material-motion-short2: 100ms;",
			)
			.xpect_contains("--io-crates-beet-ui-style-material-typography-headline-large-weight: var(--io-crates-beet-ui-style-material-typography-weight-regular);");
	}
}

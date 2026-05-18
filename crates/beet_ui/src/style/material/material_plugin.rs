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
		.with_extend(themes::from_color(color))
		.with_extend(themes::light_scheme())
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

	#[beet_core::test]
	fn material_rule_set() {
		MaterialStylePlugin::world()
			.with_state::<Res<RuleSet>, _>(|rules| {
				// OnPrimary is in the light/dark scheme rules, not the root default rule
				rules
					.iter()
					.any(|rule| rule.get(&colors::OnPrimary.into()).is_ok())
			})
			.xpect_true();
	}
	#[beet_core::test]
	fn material_css() {
		MaterialStylePlugin::world()
			.with_state::<StyleQuery, _>(|query| {
				query.build_css(&default())
			})
			.xunwrap()
			.xpect_contains(
				"--io-crates-beet-ui-style-material-motion-short2: 100ms;",
			)
			.xpect_contains("--io-crates-beet-ui-style-material-typography-headline-large-weight: var(--io-crates-beet-ui-style-material-typography-weight-regular);");
	}
}

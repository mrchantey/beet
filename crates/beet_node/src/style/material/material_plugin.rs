use super::*;
use beet_core::prelude::*;

use crate::style::RuleStore;
pub struct MaterialStylePlugin {
	color: Color,
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
		app.insert_resource(
			default_store(self.color.clone())
				.with(themes::light_scheme())
				.with(rules::hero_heading()),
		);
	}
}


/// Returns a [`RuleStore`] with all material design default values.
///
/// Applies rules in order: color tones from seed, opacities, typography,
/// shapes, elevations, durations, and motions.
pub fn default_store(color: impl Into<Color>) -> RuleStore {
	RuleStore::default()
		.with(themes::from_color(color))
		.with(themes::default_opacities())
		.with(typography::default_typography())
		.with(geometry::default_shapes())
		.with(geometry::default_elevations())
		.with(motion::default_durations())
		.with(motion::default_motions())
}

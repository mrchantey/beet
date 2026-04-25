use crate::style::material::geometry;
use crate::style::material::motion;
use crate::style::material::themes;
use crate::style::material::typography;
use crate::style::*;
use beet_core::prelude::*;

/// Returns a [`SelectorStore`] with all material design default values.
///
/// Applies selectors in order: color tones from seed, opacities, typography,
/// shapes, elevations, durations, and motions.
pub fn default_store(color: impl Into<Color>) -> SelectorStore {
	SelectorStore::default()
		.with(themes::from_color(color))
		.with(themes::default_opacities())
		.with(typography::default_typography())
		.with(geometry::default_shapes())
		.with(geometry::default_elevations())
		.with(motion::default_durations())
		.with(motion::default_motions())
}

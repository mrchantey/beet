use crate::style::TokenStore;
use crate::style::defs::*;
use beet_core::prelude::Merge;
use bevy::color::Color;




pub fn default_store(color: impl Into<Color>) -> TokenStore {
	themes::from_color(color)
		.with_merge(themes::default_opacities())
		.with_merge(default_typography())
		.with_merge(default_shape_corners())
		.with_merge(default_elevations())
		.with_merge(default_motions())
}

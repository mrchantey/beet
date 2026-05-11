use crate::style::resolve_styles;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StylePlugin;

impl Plugin for StylePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, resolve_styles.in_set(ResolveStylesSet));
	}
}



#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveStylesSet;

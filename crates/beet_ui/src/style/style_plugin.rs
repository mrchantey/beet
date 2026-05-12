use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StylePlugin;

impl Plugin for StylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<TokenPlugin>()
			.init_resource::<CssTokenMap>()
			.add_systems(PostUpdate, resolve_styles.in_set(ResolveStylesSet));
	}
}



#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveStylesSet;

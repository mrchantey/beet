use crate::content::mark_text_changed;
use crate::prelude::*;
use beet_core::prelude::*;

/// System set for propagating content changes through the entity hierarchy.
///
/// Runs in [`PostUpdate`] and ensures that child [`TextContent`](crate::content::TextContent)
/// mutations are reflected on parent [`Text`](crate::content::Text) markers
/// before downstream systems (eg TUI rebuild) observe them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropagateChanges;

#[derive(Default)]
pub struct StackPlugin;

impl Plugin for StackPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<InterfacePlugin>()
			.init_plugin::<DocumentPlugin>()
			.init_plugin::<RouterPlugin>()
			.add_systems(
				PostUpdate,
				mark_text_changed.in_set(PropagateChanges),
			);
	}
}

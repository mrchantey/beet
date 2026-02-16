use crate::nodes::node::mark_node_changed;
#[cfg(feature = "markdown")]
use crate::parsers::load_file_content;
use crate::prelude::*;
use beet_core::prelude::*;

/// System set for propagating content changes through the entity hierarchy.
///
/// Runs in [`PostUpdate`] and ensures that child [`TextNode`](crate::nodes::TextNode)
/// mutations are reflected on parent [`Node`](crate::nodes::Node) markers
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
				mark_node_changed.in_set(PropagateChanges),
			);

		#[cfg(feature = "markdown")]
		app.add_systems(PreUpdate, load_file_content);
	}
}

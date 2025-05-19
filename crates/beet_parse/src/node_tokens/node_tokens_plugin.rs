use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

/// System step for creating the entities containing nodes and their tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportNodesStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessNodesStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportNodesStep;


pub struct NodeTokensPlugin;


impl Plugin for NodeTokensPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				ProcessNodesStep.after(ImportNodesStep),
				ExtractDirectivesSet.in_set(ProcessNodesStep),
				ExportNodesStep.after(ProcessNodesStep),
			),
		)
		.add_plugins((
			tokens_to_rstml_plugin,
			rstml_to_node_tokens_plugin,
			node_tokens_to_rust_plugin,
		))
		.add_plugins((default_directives_plugin, web_directives_plugin));
	}
}

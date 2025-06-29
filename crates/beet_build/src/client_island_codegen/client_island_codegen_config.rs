use super::*;
use crate::prelude::*;
use beet_bevy::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default)]
pub struct ClientIslandCodegenPlugin;

/// Perform extra processing after files have been imported and processed.
/// - After [`ExportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct CollectClientIslandStep;



impl Plugin for ClientIslandCodegenPlugin {
	fn build(&self, app: &mut App) {
		app
			.configure_sets(
				Update,
				CollectClientIslandStep.before(ExportCodegenStep),
			)
			.add_systems(
				Update,
				(
					collect_client_islands.in_set(CollectClientIslandStep),
					compile_wasm.after(ExportCodegenStep),
				)
					.chain(),
			);
	}
}


/// The default codegen builder for a beet site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientIslandCodegenConfig {
	/// These imports will be added to the head of the wasm imports file.
	/// This will be required for any components with a client island directive.
	/// By default this will include `use beet::prelude::*;`
	#[serde(flatten)]
	pub codegen_file: CodegenFile,
	#[serde(flatten)]
	pub collect_client_islands: CollectClientIslands,
}

impl Default for ClientIslandCodegenConfig {
	fn default() -> Self {
		Self {
			codegen_file: CodegenFile::default(),
			collect_client_islands: CollectClientIslands::default(),
		}
	}
}

impl NonSendPlugin for ClientIslandCodegenConfig {
	fn build(self, app: &mut App) {
		app.world_mut()
			.spawn((self.codegen_file.sendit(), self.collect_client_islands));
	}
}

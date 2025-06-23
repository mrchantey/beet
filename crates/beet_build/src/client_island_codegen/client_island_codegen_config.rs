use super::collect_client_islands::collect_client_islands;
use crate::prelude::*;
use beet_bevy::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::prelude::ProcessRouterCodegenStep;

#[derive(Default)]
pub struct ClientIslandCodegenPlugin;


impl Plugin for ClientIslandCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			collect_client_islands.in_set(ProcessRouterCodegenStep),
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
	pub collect_client_island_plugin: CollectClientIslandPlugin,
}

impl Default for ClientIslandCodegenConfig {
	fn default() -> Self {
		Self {
			codegen_file: CodegenFile::default(),
			collect_client_island_plugin: CollectClientIslandPlugin::default(),
		}
	}
}

impl NonSendPlugin for ClientIslandCodegenConfig {
	fn build(self, app: &mut App) {
		app.world_mut().spawn((
			self.codegen_file.sendit(),
			self.collect_client_island_plugin,
		));
	}
}

use crate::prelude::*;
use beet_bevy::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The default codegen builder for a beet site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientIslandCodegenConfig {
	/// These imports will be added to the head of the wasm imports file.
	/// This will be required for any components with a client island directive.
	/// By default this will include `use beet::prelude::*;`
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

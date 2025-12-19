use crate::prelude::*;
use beet_core::prelude::*;
use beet_router::prelude::*;

#[derive(Default)]
pub struct CliPlugin;

impl Plugin for CliPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<RouterPlugin>()
			.init_plugin::<BuildPlugin>()
			.insert_resource(CargoManifest::load().unwrap())
			// temp: hardcoded until bevy scene overhaul
			.insert_resource(PackageConfig {
				title: "Beet".into(),
				binary_name: "beet_site".into(),
				version: "0.0.8-dev.9".into(),
				description: "The beet website, built with beet".into(),
				homepage: "https://beetstack.dev".into(),
				repository: Some("https://github.com/mrchantey/beet".into()),
				stage: "dev".into(),
				service_access: ServiceAccess::Local,
			})
			.add_systems(
				Update,
				// chain for determinism
				poll_child_handles,
			)
			.add_observer(interrupt_child_handles)
			// temp: hardcoded until cli args
			.insert_resource(LaunchConfig {
				package: Some("beet_site".to_string()),
				..default()
			});
	}
}

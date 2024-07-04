use anyhow::Result;
use bevy::audio::DefaultSpatialScale;
use bevy::pbr::DirectionalLightShadowMap;
use bevy::pbr::PointLightShadowMap;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use std::fs::File;
use std::fs::{
	self,
};
use std::io::Write;

pub fn save_scene(
	world: &mut World,
	filename: impl Into<String>,
) -> Result<()> {
	let filename = filename.into();
	// let scene = DynamicScene::from_world(world);
	let scene = DynamicSceneBuilder::from_world(world)
		// render plugin
		.deny_resource::<Msaa>()
		.deny_resource::<ClearColor>()
		.deny_resource::<AmbientLight>()
		.deny_resource::<DirectionalLightShadowMap>()
		.deny_resource::<PointLightShadowMap>()
		.deny_resource::<GlobalVolume>()
		.deny_resource::<DefaultSpatialScale>()
		.deny_resource::<GizmoConfigStore>()
		// time plugin
		.deny_resource::<Time>()
		.deny_resource::<Time<Real>>()
		.deny_resource::<Time<Virtual>>()
		.deny_resource::<Time<Fixed>>()
		.deny_resource::<TimeUpdateStrategy>()
		.extract_entities(world.iter_entities().map(|entity| entity.id()))
		.extract_resources()
		.build();

	let type_registry = world.resource::<AppTypeRegistry>();
	let serialized_scene = scene.serialize(&type_registry.read())?;

	if let Some(dir_path) = std::path::Path::new(&filename).parent() {
		fs::create_dir_all(dir_path)?;
	}

	let mut file = File::create(filename)?;
	file.write(serialized_scene.as_bytes())?;
	Ok(())
}

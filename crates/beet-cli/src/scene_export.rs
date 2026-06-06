//! Shared helper for the scene-exporting examples (`default_cli`,
//! `remote_loader`): resolve where the generated `beet.json` is written.
use beet::prelude::*;

/// Serialize `root`'s entity tree to JSON, write it to [`scene_output_path`],
/// then despawn `root` and exit. Shared by the scene-exporting examples.
pub fn export_scene(world: &mut World, root: Entity) -> Result {
	let json = WorldSerdeSaver::new(world)
		.with_entity_tree(root)
		.save(MediaType::Json)?
		.as_utf8()?
		.to_string();
	world.entity_mut(root).despawn();

	let path = scene_output_path()?;
	fs_ext::write(&path, &json)?;
	cross_log!("wrote scene to {path}");
	world.write_message(AppExit::Success);
	Ok(())
}

/// Where an exported scene is written: the `--output <path>` CLI argument, or
/// [`BEET_SCENE_FILE`] in the cwd when omitted.
fn scene_output_path() -> Result<AbsPathBuf> {
	let args = CliArgs::parse_env();
	let path = args
		.params
		.get("output")
		.map(String::as_str)
		.unwrap_or(BEET_SCENE_FILE);
	AbsPathBuf::new(path)?.xok()
}

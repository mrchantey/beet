use crate::prelude::*;
use beet_utils::prelude::AbsPathBuf;
use beet_utils::prelude::PathExt;
use bevy::prelude::*;
use std::path::PathBuf;


/// A file that belongs to a [`FileGroup`], spawned as its child.
/// The number of child [`FileRouteTokens`] that will be spawned
/// will be either 1 or 0..many, depending on whether the file
/// is a 'single file route':
/// - `foo.md`: 1
/// - `foo.rs`: 0 or more
/// - `foo.rsx: 0 or more
#[derive(Component)]
pub struct RouteFile {
	/// The index of the file in the group, used for generating unique identifiers.
	pub index: usize,
	/// The absolute path to the file.
	pub abs_path: AbsPathBuf,
	/// The local path relative to the [`FileGroup::src`],
	/// used to generate the route path.
	pub local_path: PathBuf,
}

/// Search the directory of each [`FileGroup`] and parse each file
pub fn spawn_route_files(
	mut commands: Commands,
	query: Populated<(Entity, &FileGroup), Added<FileGroup>>,
) -> Result {
	for (entity, group) in query.iter() {
		let mut entity = commands.entity(entity);
		for (index, abs_path) in group.collect_files()?.into_iter().enumerate()
		{
			let local_path = PathExt::create_relative(&group.src, &abs_path)?;

			entity.with_child(RouteFile {
				index,
				abs_path,
				local_path,
			});
		}
	}
	Ok(())
}

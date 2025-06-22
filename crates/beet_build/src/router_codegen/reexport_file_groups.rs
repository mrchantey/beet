// use crate::prelude::*;
// use bevy::prelude::*;




// pub fn reexport_file_groups(
// 		mut commands: Commands,
// 		file_groups: Query<(Entity, &FileGroup)>,
// ) {
// 		for (entity, file_group) in file_groups.iter() {
// 				// Re-export the file group as a new entity
// 				commands.entity(entity).insert(ReexportedFileGroup(file_group.clone()));
// 		}
// }
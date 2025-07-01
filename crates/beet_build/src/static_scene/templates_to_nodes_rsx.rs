use crate::prelude::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


/// For a given rsx file, insert as [`CombinatorToNodeTokens`].
pub fn templates_to_nodes_rsx(
	mut commands: Commands,
	query: Populated<(Entity, &SourceFile), Added<SourceFile>>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "rsx"
		{
			let file = ReadFile::to_string(path)?;

			commands.spawn((
				ChildOf(entity),
				StaticRoot,
				MacroIdx::new(path.into_ws_path()?, LineCol::default()),
				CombinatorTokens::new(file),
			));
		}
	}
	Ok(())
}

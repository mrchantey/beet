use crate::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


/// For a given rsx file, insert as [`CombinatorToNodeTokens`].
pub fn templates_to_nodes_rsx(
	mut commands: Commands,
	query: Populated<(Entity, &TemplateFile), Changed<TemplateFile>>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "rsx"
		{
			let file = ReadFile::to_string(path.into_abs())?;

			commands.spawn((
				ChildOf(entity),
				TemplateKey::new(path.path().clone(), 0),
				SourceFile::new(path.path().clone()),
				CombinatorTokens(file),
			));
		}
	}
	Ok(())
}

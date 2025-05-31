use crate::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;
use sweet::prelude::ReadFile;


/// For a given markdown file, parse to valid rsx combinator syntax and insert
/// as [`CombinatorToNodeTokens`].
pub fn templates_to_nodes_md(
	mut commands: Commands,
	query: Populated<(Entity, &TemplateFile), Changed<TemplateFile>>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "md"
		{
			let file = ReadFile::to_string(path.into_abs_unchecked())?;
			let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file);

			commands.spawn((
				TemplateSource(entity),
				TemplateKey::new(path.path().clone(), 0),
				SourceFile::new(path.path().clone()),
				CombinatorToNodeTokens(rsx_str),
			));
		}
	}
	Ok(())
}

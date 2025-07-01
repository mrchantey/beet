use crate::prelude::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;

/// For a given markdown file, parse to valid rsx combinator syntax and insert
/// as [`CombinatorToNodeTokens`].
pub fn templates_to_nodes_md(
	mut commands: Commands,
	query: Populated<
		(Entity, &SourceFile),
		(With<SnippetFile>, Changed<SourceFile>),
	>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "md"
		{
			commands.entity(entity).despawn_related::<Children>();
			let file = ReadFile::to_string(path)?;
			let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file);

			commands.spawn((
				ChildOf(entity),
				RsxSnippetRoot,
				MacroIdx::new(path.into_ws_path()?, LineCol::default()),
				CombinatorTokens::new(rsx_str),
			));
		}
	}
	Ok(())
}

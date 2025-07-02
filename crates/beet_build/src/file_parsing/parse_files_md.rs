use crate::prelude::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;

/// For a given markdown file, parse to valid rsx combinator syntax and insert
/// as [`CombinatorToNodeTokens`].
pub fn parse_files_md(
	mut commands: Commands,
	query: Populated<(Entity, &SourceFile), Changed<SourceFile>>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
		// TODO md should not 
			&& (ex == "md" || ex == "mdx")
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::as_beet::render_fragment;
	use beet_utils::prelude::WsPathBuf;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn parse_md() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default());
		let entity = app
			.world_mut()
			.spawn(SourceFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/test_docs/hello.md",
				)
				.into_abs(),
			))
			.id();

		app.update();
		let child = app.world().entity(entity).get::<Children>().unwrap()[0];
		app.world_mut()
			.run_system_once_with(render_fragment, child)
			.unwrap()
			.xpect()
			// only the output of the snippet, not the instance
			.to_be("<h1>Hello</h1><p>This page is all about saying hello</p><main>## Nested Heading\n\tnested markdown doesnt work yet</main>");
	}
	#[test]
	fn parse_mdx() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default());
		let entity = app
			.world_mut()
			.spawn(SourceFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/test_docs/index.mdx",
				)
				.into_abs(),
			))
			.id();

		app.update();
		let child = app.world().entity(entity).get::<Children>().unwrap()[0];
		app.world_mut()
			.run_system_once_with(render_fragment, child)
			.unwrap()
			.xpect()
			// only the output of the snippet, not the instance
			.to_be(
				"<h1>Docs</h1><p>Docs are good for your health</p><div>1 + 1 is</div>",
			);
	}
}

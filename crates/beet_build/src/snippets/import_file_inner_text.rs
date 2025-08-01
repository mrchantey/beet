use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;



/// We cant use `include_str!` via static analysis so manually
/// import the file contents and replace the `FileInnerText`
/// with an `InnerText` containing the file contents.
pub fn import_file_inner_text(
	mut commands: Commands,
	query: Populated<(Entity, &FileInnerText)>,
	parents: Query<&ChildOf>,
	snippets_of: Query<&RsxSnippetOf>,
	source_files: Query<(Entity, &SourceFile, Option<&SourceFileRefTarget>)>,
) -> Result<()> {
	for (entity, file) in query.iter() {
		let snippet_of = snippets_of.get(parents.root_ancestor(entity))?;
		let (source_file_ent, source_file, target) =
			source_files.get(snippet_of.0)?;
		let path = source_file.parent().unwrap_or_default().join(&file.0);
		let contents = ReadFile::to_string(&path)?;

		// 1. change the FileInnerText to InnerText
		commands
			.entity(entity)
			.remove::<FileInnerText>()
			// here we are directly inserting the content, unlike [`FileInnerText::self_tokens`]
			// which feature gates the import for smaller client bundles.
			// this is ok because this method is only used for live reloading
			.insert(InnerText::new(contents));

		// 2. link the source files
		if false
			== target
				.map(|target| {
					target
						.iter()
						.filter_map(|ent| source_files.get(ent).ok())
						.any(|(_, file, _)| **file == path)
				})
				.unwrap_or(false)
		{
			commands
				.spawn((SourceFileRef(source_file_ent), SourceFile::new(path)));
		}
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	// this file will be parsed, declare an fs src
	#[allow(unexpected_cfgs)]
	fn _foobar() { let _ = rsx! {<style src="../../tests/test_file.css"/>}; }


	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin);
		let file = app
			.world_mut()
			.spawn(SourceFile::new(
				AbsPathBuf::new_workspace_rel(file!()).unwrap(),
			))
			.id();
		app.update();

		#[cfg(feature = "css")]
		let expected = "body[data-beet-style-id-15207297232399335040] {\n  color: #00f;\n}\n";
		#[cfg(not(feature = "css"))]
		let expected = include_str!("../../tests/test_file.css");

		app.world_mut().query_once::<&InnerText>()[0]
			.xpect()
			.to_be(&InnerText::new(expected));

		// links source files
		app.world_mut().query_once::<&SourceFileRef>()[0]
			.0
			.xpect()
			.to_be(file);

		app.world_mut()
			.query_once::<&SourceFileRef>()
			.len()
			.xpect()
			.to_be(1);
		app.update();
		// second update does not spawn a new SourceFileRef
		app.world_mut()
			.query_once::<&SourceFileRef>()
			.len()
			.xpect()
			.to_be(1);
	}
}

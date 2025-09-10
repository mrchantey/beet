use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;



/// We cant use `include_str!` via static analysis so manually
/// import the file contents and replace the `FileInnerText`
/// with an `InnerText` containing the file contents.
pub fn import_file_inner_text(
	mut commands: Commands,
	query: Populated<(Entity, &FileInnerText), Added<FileInnerText>>,
	parents: Query<&ChildOf>,
	source_files: Query<(Entity, &SourceFile, Option<&WatchedFiles>)>,
) -> Result<()> {
	for (entity, file_text) in query.iter() {
		// get the parent source file
		let (source_file_ent, source_file, watched_files) = parents
			.iter_ancestors(entity)
			.find_map(|en| source_files.get(en).ok())
			.ok_or_else(|| {
				bevyhow!("FileInnerText has no SourceFile parent: {entity:?}")
			})?;
		let path = source_file.parent().unwrap_or_default().join(&file_text.0);
		let contents = ReadFile::to_string(&path)?;

		// 1. change the FileInnerText to InnerText
		commands
			.entity(entity)
			.remove::<FileInnerText>()
			// here we are directly inserting the content, unlike [`FileInnerText::self_tokens`]
			// which feature gates the import for smaller client bundles.
			// this is ok because this method is only used for live reloading
			.insert(InnerText::new(contents));

		// 2. ensure the file is being watched
		if false
			== watched_files
				.map(|children| {
					children
						.iter()
						.filter_map(|ent| source_files.get(ent).ok())
						.any(|(_, file, _)| **file == path)
				})
				.unwrap_or(false)
		{
			if let Some((watched_entity, _, _)) =
				source_files.iter().find(|(_, file, _)| ***file == path)
			{
				commands
					.entity(watched_entity)
					// TODO many-many relations
					.insert(FileWatchedBy(source_file_ent));
			} else {
				warn!(
					"file included by 'src=..' tag not found, changes will not be watched: {path:?}"
				);
			}
		}
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::*;
	use beet_rsx::prelude::*;
	use beet_utils::prelude::*;
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
		let expected = "body[data-beet-style-id-PLACEHOLDER] {\n  color: #00f;\n}\n";
		#[cfg(not(feature = "css"))]
		let expected = include_str!("../../tests/test_file.css");

		app.world_mut().query_once::<&InnerText>()[0]
			.xpect_eq(InnerText::new(expected));

		// links source files, child index flaky?
		app.world_mut().query_once::<&ChildOf>()[0].0.xpect_eq(file);

		app.world_mut().query_once::<&ChildOf>().len().xpect_eq(2);
		app.update();
		// second update does not spawn a new ChildOf
		app.world_mut().query_once::<&ChildOf>().len().xpect_eq(2);
	}
}

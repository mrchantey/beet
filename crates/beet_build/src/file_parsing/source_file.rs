use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_fs::process::WatchEvent;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::prelude::*;
use std::path::Path;

/// Adde to an entity used to represent an file included
/// in the [`WorkspaceConfig`]. These are loaded for different
/// purposes by [`FileSnippetPlugin`] and [`RouteCodegenPlugin`].
#[derive(Debug, Clone, PartialEq, Eq, Component, Deref)]
// #[component(immutable)]
#[require(FileExprHash)]
pub struct SourceFile {
	path: AbsPathBuf,
}

impl SourceFile {
	pub fn new(path: AbsPathBuf) -> Self { Self { path } }
	pub fn path(&self) -> &AbsPathBuf { &self.path }
}
impl AsRef<Path> for SourceFile {
	fn as_ref(&self) -> &Path { self.path.as_ref() }
}



/// Create a [`SourceFile`] for each file specified in the [`WorkspaceConfig`].
/// This will run once for the initial load, afterwards [`handle_changed_files`]
/// will incrementally load changed files.
#[cfg_attr(test, allow(dead_code))]
pub(super) fn load_workspace_source_files(
	mut commands: Commands,
	config: When<Res<WorkspaceConfig>>,
) -> bevy::prelude::Result {
	commands.spawn((Children::spawn(SpawnIter(
		config
			.get_files()?
			.into_iter()
			.map(|path| SourceFile::new(path)),
	)),));
	Ok(())
}



/// Notify bevy Mutation system that a file has changed.
pub(super) fn parse_file_watch_events(
	mut events: EventReader<WatchEvent>,
	config: When<Res<WorkspaceConfig>>,
	mut query: Query<&mut SourceFile>,
) -> bevy::prelude::Result {
	for ev in events
		.read()
		// we only care about files that a builder will want to save
		.filter(|ev| config.passes(&ev.path))
	{
		tracing::debug!("SourceFile Changed: {}", ev.path.display());
		for mut file in query.iter_mut().filter(|file| ***file == ev.path) {
			file.set_changed();
		}
	}
	Ok(())
}

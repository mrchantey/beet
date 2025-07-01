use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_fs::process::WatchEvent;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::path::Path;

/// Adde to an entity used to represent an file included
/// in the [`WorkspaceConfig`]. These are loaded for different
/// purposes by [`StaticScenePlugin`] and [`RouteCodegenPlugin`].
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


/// Notify bevy Mutation system that a file has changed.
pub(super) fn touch_changed_source_files(
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

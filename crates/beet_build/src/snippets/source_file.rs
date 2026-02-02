//! Source file management for the build system.
//!
//! This module handles the representation and lifecycle of source files
//! that are tracked by the build system, including responding to file
//! system events.

use crate::prelude::*;
use beet_core::exports::notify::EventKind;
use beet_core::exports::notify::event::CreateKind;
use beet_core::exports::notify::event::ModifyKind;
use beet_core::exports::notify::event::RemoveKind;
use beet_core::prelude::*;
use std::path::Path;

/// Represents a source file included in the [`WorkspaceConfig`].
///
/// These entities are loaded for different purposes by the snippets system
/// and the route codegen system.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref)]
#[require(FileExprHash)]
pub struct SourceFile {
	/// The absolute path to this source file.
	path: AbsPathBuf,
}

impl SourceFile {
	/// Creates a new source file reference from an absolute path.
	pub fn new(path: AbsPathBuf) -> Self { Self { path } }

	/// Returns a reference to the source file's path.
	pub fn path(&self) -> &AbsPathBuf { &self.path }
}

impl AsRef<Path> for SourceFile {
	fn as_ref(&self) -> &Path { self.path.as_ref() }
}


/// Parent entity for all [`SourceFile`] entities that exist outside
/// of a [`RouteFileCollection`].
#[derive(Component)]
pub struct NonCollectionSourceFiles;

/// A [`SourceFile`] watched by another [`SourceFile`], for example
/// a file with an `include_str!()`;
///
/// This relationship enables cascading updates when dependent files change.
#[derive(Deref, Component)]
#[relationship(relationship_target = WatchedFiles)]
// TODO many-many relations
pub struct FileWatchedBy(pub Entity);


/// A collection of [`SourceFile`] entities that this [`SourceFile`] is watching.
///
/// If any child file changes, the parent should also be marked as changed.
#[derive(Deref, Component)]
#[relationship_target(relationship = FileWatchedBy, linked_spawn)]
pub struct WatchedFiles(Vec<Entity>);


/// Updates [`SourceFile`] entities based on file system watch events.
///
/// This observer handles:
/// - **Create**: Spawns new [`SourceFile`] entities
/// - **Remove**: Despawns matching [`SourceFile`] entities
/// - **Modify**: Marks files as changed and resets their children
/// - **Rename**: Handles both the "from" and "to" sides of renames
pub fn parse_dir_watch_events(
	ev: On<DirEvent>,
	mut commands: Commands,
	root_entity: Populated<Entity, With<NonCollectionSourceFiles>>,
	config: When<Res<WorkspaceConfig>>,
	mut existing: Query<(Entity, &mut SourceFile)>,
) -> Result {
	for ev in ev
		.iter()
		// we only care about files specified in the config
		.filter(|ev| config.passes(&ev.path))
	{
		tracing::debug!("SourceFile event: {}", ev);

		let matches = existing
			.iter_mut()
			.filter(|(_, file)| ***file == ev.path)
			.map(|(en, _)| en)
			.collect::<Vec<_>>();

		match ev.kind {
			EventKind::Create(CreateKind::File) => {
				commands.spawn((
					ChildOf(root_entity.single()?),
					SourceFile::new(ev.path.clone()),
				));
			}
			EventKind::Create(CreateKind::Folder) => {
				// noop
			}
			// emitted for both the from and to renames so
			// assume if no matches, its the To event
			EventKind::Modify(ModifyKind::Name(_)) => {
				if matches.is_empty() {
					commands.spawn((
						ChildOf(root_entity.single()?),
						SourceFile::new(ev.path.clone()),
					));
				} else {
					for entity in matches {
						commands.entity(entity).despawn();
					}
				}
			}
			EventKind::Remove(RemoveKind::File) => {
				for entity in matches {
					commands.entity(entity).despawn();
				}
			}
			EventKind::Modify(_) => {
				for entity in matches {
					commands.run_system_cached_with(reset_file, entity);
				}
			}
			other => {
				tracing::warn!("Unhandled file event: {:?}", other);
			}
		}
	}
	Ok(())
}


/// Resets a [`SourceFile`] when it changes.
///
/// This function:
/// - Marks the file as added (triggering re-processing)
/// - Removes all children entities
/// - Recursively resets any parent files that watch this file
fn reset_file(
	In(entity): In<Entity>,
	mut commands: Commands,
	mut files: Query<(Entity, &mut SourceFile, Option<&FileWatchedBy>)>,
) {
	if let Ok((entity, mut file, parent)) = files.get_mut(entity) {
		trace!("Resetting Source File: {}", file.path);
		file.set_added();
		commands.entity(entity).despawn_related::<Children>();
		if let Some(parent) = parent {
			commands.run_system_cached_with(reset_file, **parent);
		}
	}
}

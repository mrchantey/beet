use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::exports::notify::EventKind;
use beet_utils::exports::notify::event::CreateKind;
use beet_utils::exports::notify::event::ModifyKind;
use beet_utils::exports::notify::event::RemoveKind;
use beet_utils::prelude::WatchEvent;
use beet_utils::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::prelude::*;
use std::path::Path;

/// Adde to an entity used to represent an file included
/// in the [`WorkspaceConfig`]. These are loaded for different
/// purposes by [`SnippetsPlugin`] and [`RouteCodegenSequence`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref)]
// #[component(immutable)]
#[require(FileExprHash)]
pub struct SourceFile {
	/// The path to this source file
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
/// This will run once for the initial load, afterwards [`parse_file_watch_events`]
/// will incrementally load changed files.
///
/// These files are initially loaded as children of the [`SourceFileRoot`],
/// but may be moved to a [`RouteFileCollection`] if the path matches.
#[cfg_attr(test, allow(dead_code))]
pub fn load_workspace_source_files(
	mut commands: Commands,
	config: When<Res<WorkspaceConfig>>,
) -> bevy::prelude::Result {
	commands.spawn((
		NonCollectionSourceFiles,
		Children::spawn(SpawnIter(
			config
				.get_files()?
				.into_iter()
				.map(|path| SourceFile::new(path)),
		)),
	));
	Ok(())
}

/// Parent of every [`SourceFile`] entity that exists outside of a [`RouteFileCollection`].
#[derive(Component)]
pub struct NonCollectionSourceFiles;

/// A [`SourceFile`] watched by another [`SourceFile`]
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = WatchedFiles)]
// TODO many-many relations
pub struct FileWatchedBy(pub Entity);


/// A collection of [`SourceFile`] entities that this [`SourceFile`] is watching.
/// If any child changes this should also change.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = FileWatchedBy, linked_spawn)]
pub struct WatchedFiles(Vec<Entity>);


/// Update [`SourceFile`] entities based on file watch events,
/// including marking as [`Changed`] on modification.
pub fn parse_file_watch_events(
	mut commands: Commands,
	mut events: EventReader<WatchEvent>,
	root_entity: Query<Entity, With<NonCollectionSourceFiles>>,
	config: When<Res<WorkspaceConfig>>,
	mut existing: Query<(Entity, &mut SourceFile)>,
) -> bevy::prelude::Result {
	for ev in events
		.read()
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


/// Runs for any [`SourceFile`] that changes:
/// - mark it as [`Added`]
/// - remove all [`Children`]
/// If it has a [`FileWatchedBy`] component, also run for that parent
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

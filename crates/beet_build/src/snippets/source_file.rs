use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::exports::notify::EventKind;
use beet_utils::exports::notify::event::CreateKind;
use beet_utils::exports::notify::event::RemoveKind;
use beet_utils::prelude::WatchEvent;
use beet_utils::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::prelude::*;
use std::path::Path;

/// Adde to an entity used to represent an file included
/// in the [`WorkspaceConfig`]. These are loaded for different
/// purposes by [`SnippetsPlugin`] and [`RouteCodegenSequence`].
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

/// Types like [`RouteFile`] and files added via include_str!
/// exist outside of the [`SourceFile`] tree,
/// but need to reference it to get its rsx children.
/// This entity will be despawned when the [`SourceFile`] is despawned.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = SourceFileRefTarget)]
pub struct SourceFileRef(pub Entity);

/// All references to this [`SourceFile`]
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = SourceFileRef,linked_spawn)]
pub struct SourceFileRefTarget(Vec<Entity>);


/// Reference to the [`SourceFile`] of this rsx snippet.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = RsxSnippets)]
pub struct RsxSnippetOf(pub Entity);

/// Rsx snippets of this [`SourceFile`], we use non-parent relations
/// to avoid missing parent in fine-grained scene exports.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = RsxSnippetOf,linked_spawn)]
pub struct RsxSnippets(Vec<Entity>);

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
		SourceFileRoot,
		Children::spawn(SpawnIter(
			config
				.get_files()?
				.into_iter()
				.map(|path| SourceFile::new(path)),
		)),
	));
	Ok(())
}

/// Parent of every [`SourceFile`] entity, any changed child [`SourceFile`]
/// will result in this being marked [`Changed`]
#[derive(Component)]
pub struct SourceFileRoot;


/// Update [`SourceFile`] entities based on file watch events,
/// including marking as [`Changed`] on modification.
pub fn parse_file_watch_events(
	mut commands: Commands,
	mut events: EventReader<WatchEvent>,
	root_entity: Query<Entity, With<SourceFileRoot>>,
	config: When<Res<WorkspaceConfig>>,
	mut existing: Query<(Entity, &mut SourceFile)>,
) -> bevy::prelude::Result {
	for ev in events
		.read()
		// we only care about files that a builder will want to save
		.filter(|ev| config.passes(&ev.path))
	{
		tracing::debug!("SourceFile event: {}", ev);

		let matches =
			existing.iter_mut().filter(|(_, file)| ***file == ev.path);

		match ev.kind {
			EventKind::Create(CreateKind::File) => {
				commands.spawn((
					ChildOf(root_entity.single()?),
					SourceFile::new(ev.path.clone()),
				));
			}
			EventKind::Remove(RemoveKind::File) => {
				for (entity, _) in matches {
					commands.entity(entity).despawn();
				}
			}
			EventKind::Modify(_) => {
				for (entity, mut file) in matches {
					file.set_changed();
					commands.run_system_cached_with(
						propagate_source_file_changes,
						entity,
					);
				}
			}
			other => {
				tracing::warn!("Unhandled file event: {:?}", other);
			}
		}
	}
	Ok(())
}


// if a [`SourceFile`] is changed, notify source files that depend
// on it.
fn propagate_source_file_changes(
	In(entity): In<Entity>,
	query: Query<&SourceFileRef>,
	mut files: Query<&mut SourceFile>,
) {
	if let Ok(ref_target) = query.get(entity)
		&& let Ok(mut file) = files.get_mut(ref_target.0)
	{
		file.set_changed();
	}
}

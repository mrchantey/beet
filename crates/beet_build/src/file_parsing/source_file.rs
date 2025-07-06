use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_fs::exports::notify::EventKind;
use beet_fs::exports::notify::event::CreateKind;
use beet_fs::exports::notify::event::RemoveKind;
use beet_fs::process::WatchEvent;
use beet_rsx::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::prelude::*;
use std::path::Path;

/// Adde to an entity used to represent an file included
/// in the [`WorkspaceConfig`]. These are loaded for different
/// purposes by [`SnippetsPlugin`] and [`RouteCodegenPlugin`].
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

/// Types like [`RouteFile`] exist outside of the [`SourceFile`] tree,
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


/// Parent of every [`SourceFile`] entity, any changed child [`SourceFile`]
/// will result in this being marked [`Changed`]
#[derive(Component)]
pub struct SourceFileRoot;

/// Create a [`SourceFile`] for each file specified in the [`WorkspaceConfig`].
/// This will run once for the initial load, afterwards [`handle_changed_files`]
/// will incrementally load changed files.
#[cfg_attr(test, allow(dead_code))]
pub(super) fn load_workspace_source_files(
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



/// Update [`SourceFile`] entities based on file watch events,
/// including marking as [`Changed`] on modification.
pub(super) fn parse_file_watch_events(
	mut commands: Commands,
	mut events: EventReader<WatchEvent>,
	config: When<Res<WorkspaceConfig>>,
	mut roots: Query<(Entity, &mut SourceFileRoot)>,
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

		for (_, mut src_file_root) in roots.iter_mut() {
			src_file_root.set_changed();
		}

		match ev.kind {
			EventKind::Create(CreateKind::File) => {
				for (root_entity, _) in roots.iter() {
					commands.spawn((
						ChildOf(root_entity),
						SourceFile::new(ev.path.clone()),
					));
				}
			}
			EventKind::Remove(RemoveKind::File) => {
				for (entity, _) in matches {
					commands.entity(entity).despawn();
				}
			}
			EventKind::Modify(_) => {
				for (_, mut file) in matches {
					file.set_changed();
				}
			}
			other => {
				tracing::warn!("Unhandled file event: {:?}", other);
			}
		}
	}
	Ok(())
}

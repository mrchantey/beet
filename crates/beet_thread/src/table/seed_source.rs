//! A seed post whose text is read by the entry creating its thread.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A seed post whose text is read by the entry creating its thread: a
/// procedure kept in a file, or a route's answer computed from the request
/// entering.
///
/// Authored as a child of its `<CreateActor>` like a [`SeedPost`], ie
/// `<CreatePost src="seeds/system.md"/>`, which reads the file at `src` in the
/// store at or above the thread. A downstream source is a template returning
/// one, its action reading whatever it needs.
///
/// Building the template reads nothing. [`Thread::enter`] resolves every
/// source of a thread it creates, in author order, calling its action on the
/// source's own entity with the [`ThreadEntry`], so it resolves
/// what it reads by ancestry, and its text lands as a [`SeedPost`] in its
/// place. A continued thread reads no source: its first posts are stored.
#[derive(Clone, Component)]
pub struct SeedSource {
	/// Names the source in an error, ie `<CreatePost src="seeds/system.md">`.
	pub label: SmolStr,
	/// Reads the seed's text for the entry creating the thread, called on
	/// the source's entity.
	pub action: Action<ThreadEntry, String>,
	/// The seed post's intent.
	pub intent: PostIntent,
	/// The seed post's status.
	pub status: PostStatus,
}

impl SeedSource {
	/// A completed [`PostIntent::Message`] seed whose text `action` reads, named by `label`.
	pub fn new(
		label: impl Into<SmolStr>,
		action: Action<ThreadEntry, String>,
	) -> Self {
		Self {
			label: label.into(),
			action,
			intent: PostIntent::Message,
			status: PostStatus::Completed,
		}
	}

	/// A seed reading the text of the file at `path` in the store at or above
	/// the thread.
	pub fn blob(path: RelPath) -> Self {
		let label = format!("<CreatePost src=\"{path}\">");
		Self::new(
			label,
			Action::new_async(
				async move |cx: ActionContext<ThreadEntry>| -> Result<String> {
					let store = cx
						.caller
						.with_state::<AncestorQuery<&BlobStore>, _>(
							|entity, query| query.get(entity).cloned(),
						)
						.await?
						.map_err(|_| {
							bevyhow!(
								"no store above the thread to read the seed `{path}`"
							)
						})?;
					store
						.get(&path)
						.await?
						.to_vec()
						.xmap(String::from_utf8)?
						.xok()
				},
			),
		)
	}

	/// Resolve every source at or under the thread `entity` in author order,
	/// each landing as the [`SeedPost`] it stood for.
	///
	/// ## Errors
	///
	/// Errors on the first source that cannot be read, naming it.
	pub(crate) async fn resolve_all(
		world: &AsyncWorld,
		entity: Entity,
		entry: &ThreadEntry,
	) -> Result {
		let sources = world
			.with_state::<(Query<&Children>, Query<(), With<SeedSource>>), _>(
				move |(children, sources)| {
					children
						.iter_descendants_depth_first(entity)
						.filter(|child| sources.contains(*child))
						.collect::<Vec<_>>()
				},
			)
			.await;
		for source in sources {
			Self::resolve(&world.entity(source), entry.clone()).await?;
		}
		Ok(())
	}

	/// Read the text and land it as the [`SeedPost`] this source stood for.
	async fn resolve(entity: &AsyncEntity, entry: ThreadEntry) -> Result {
		let source = entity.get(|source: &SeedSource| source.clone()).await?;
		let text = entity
			.call_detached(source.action.clone(), entry)
			.await
			.map_err(|err| {
				bevyhow!("the seed {} could not be read: {err}", source.label)
			})?;
		entity
			.with(move |mut entity: EntityWorldMut| {
				entity.remove::<SeedSource>().insert(Post::spawn_with(
					text,
					source.intent,
					source.status,
				));
			})
			.await?;
		Ok(())
	}
}

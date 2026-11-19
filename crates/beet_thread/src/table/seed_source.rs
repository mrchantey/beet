//! A seed post whose text is read while its thread builds.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A seed post whose text is read while its thread builds: a procedure kept
/// in a file, or text computed from the request a thread is built for.
///
/// Authored as a child of its `<CreateActor>` like a [`SeedPost`], ie
/// `<CreatePost src="seeds/system.md"/>`, which reads the file at `src` in the
/// store at or above the thread. A downstream source is a template returning
/// one, its action reading whatever it needs.
///
/// The read is a pending dependency of the build, parked on the build root
/// as an include is, so `Ready` waits for it. Its action is called on the
/// source's own entity, resolving what it reads by ancestry, and its text
/// lands as a [`SeedPost`] in its place. The thread reduces only once every
/// source has landed ([`ThreadWindow::reduce`]). Only a fresh thread keeps
/// what it read: a continued thread enters with its stored seed
/// ([`Thread::enter`]).
///
/// A failed read leaves the source in place and a [`TemplateError`] on the
/// build root, so the thread never reduces and its entry names the seed.
#[derive(Clone, Component)]
#[component(on_add = SeedSource::on_add)]
pub struct SeedSource {
	/// Names the source in a settle report or an error, ie
	/// `<CreatePost src="seeds/system.md">`.
	pub label: SmolStr,
	/// Reads the seed's text, called on the source's entity.
	pub action: Action<(), String>,
	/// The seed post's intent.
	pub intent: PostIntent,
	/// The seed post's status.
	pub status: PostStatus,
}

impl SeedSource {
	/// A completed, OK seed whose text `action` reads, named by `label`.
	pub fn new(label: impl Into<SmolStr>, action: Action<(), String>) -> Self {
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
				async move |cx: ActionContext| -> Result<String> {
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

	/// Park the read on the build root (or on this entity outside a build)
	/// and run it as a task holding the guard, so however it ends the guard
	/// resolves, as a `<RoutesDir>` scan does.
	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let entity = cx.entity;
		let root = world.get_resource::<TemplateBuildRoot>().map(|root| **root);
		let Some(label) = world
			.get::<SeedSource>(entity)
			.map(|source| source.label.clone())
		else {
			return;
		};
		world.commands().queue(move |world: &mut World| {
			let guard = TemplatePending::park_on(
				world,
				root.unwrap_or(entity),
				PendingKind::Passive,
				label,
			);
			let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
				// despawned before the command ran: the dropped guard resolves
				// through the sweep
				return;
			};
			entity_mut.run_async_local(async move |entity: AsyncEntity| {
				let root = guard.root();
				let result = Self::resolve(&entity).await;
				entity
					.world()
					.with(move |world: &mut World| {
						if let (Err(err), Ok(mut root)) =
							(result, world.get_entity_mut(root))
						{
							root.insert(TemplateError::new(err));
						}
						guard.resolve(world);
					})
					.await;
			});
		});
	}

	/// Read the text and land it as the [`SeedPost`] this source stood for.
	async fn resolve(entity: &AsyncEntity) -> Result {
		let source = entity.get(|source: &SeedSource| source.clone()).await?;
		let text = entity.call_detached(source.action.clone(), ()).await?;
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A thread seeding its system post from `seeds/system.md`, pinned actor
	/// ids so only the seed's text moves its hash.
	const SCENE: &str = r#"
<Thread {Sequence}>
	<CreateActor name="System" kind="System" id=1><CreatePost src="seeds/system.md"/></CreateActor>
	<CreateActor name="Agent" kind="Agent" id=2 {MockPostStreamer}/>
</Thread>
"#;

	/// Build [`SCENE`] under a root holding `store` and enter it, answering
	/// the world and the thread, or the entry's error.
	async fn enter(store: &BlobStore) -> (App, Result<Entity>) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let root = app.world_mut().spawn(store.clone()).id();
		let thread = BsxTemplate::parse_entry(app.world(), SCENE)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.world_mut().entity_mut(thread).insert(ChildOf(root));
		// the read is still in flight, so the thread has not reduced
		ThreadWindow::reduce_now(app.world_mut());
		app.world()
			.get::<ThreadWindow>(thread)
			.is_none()
			.xpect_true();
		let result = app
			.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				TemplatePending::settle(&world).await;
				Thread::enter(world, thread, default())
					.await
					.map(|_| thread)
			})
			.await;
		(app, result)
	}

	/// A `src` seed is read while its thread builds, landing as the thread's
	/// first seed post.
	#[beet_core::test]
	async fn a_src_seed_is_read_at_build() {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::new("seeds/system.md"), "be brief".to_string())
			.await
			.unwrap();
		let (app, thread) = enter(&store).await;
		let thread = thread.unwrap();
		let window = app.world().get::<ThreadWindow>(thread).unwrap();
		window.posts()[0].to_string().xpect_eq("be brief");
		window.seed().contains(&window.posts()[0].id()).xpect_true();
	}

	/// A seed that cannot be read leaves its thread unreduced, and entering it
	/// names the seed and why.
	#[beet_core::test]
	async fn a_missing_src_names_the_seed() {
		enter(&BlobStore::temp())
			.await
			.1
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"before its seeds resolved (<CreatePost src=\"seeds/system.md\">)",
			);
	}
}

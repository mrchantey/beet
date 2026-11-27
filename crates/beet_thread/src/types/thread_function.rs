//! A thread run as a function, see [`ThreadFunction`].
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// A thread run as a function: built from a template for one call, entered,
/// run to its outcome and despawned, nobody typing. A [`ThreadRoute`] runs
/// one per request and a [`Compact`] one per compaction.
///
/// Dropped before [`Self::despawn`], ie its call cancelled by an interrupt,
/// it despawns its build in a task of its own, so a dropped call never leaves
/// a thread behind.
pub(crate) struct ThreadFunction {
	world: AsyncWorld,
	/// The build root, the container holding the template's own root.
	root: Entity,
	/// The template's name, for errors.
	name: String,
	spawner: AsyncSpawner,
	despawned: bool,
}

/// What a [`ThreadFunction`] ran to.
pub(crate) struct ThreadOutcome {
	/// The behavior's outcome.
	pub outcome: Outcome,
	/// The template's first thread, still spawned until the function
	/// despawns.
	pub thread: Entity,
}

impl ThreadFunction {
	/// Build `source` for the request `parts`, the build reading `anchor`'s
	/// url space, settle it and attach it beneath `parent`, where it resolves
	/// its ancestor stores and the surrounding url space as if authored there.
	/// It is built detached first, so the surrounding [`RouteTree`] never
	/// rebuilds for it.
	///
	/// ## Errors
	///
	/// Errors when the build fails, the build despawned.
	pub async fn build(
		world: &AsyncWorld,
		anchor: Entity,
		parent: Option<Entity>,
		source: LoadedTemplate,
		parts: RequestParts,
	) -> Result<Self> {
		let name = source.name().to_string();
		let (root, spawner) = world
			.with(move |world: &mut World| -> Result<(Entity, AsyncSpawner)> {
				let router = world
					.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
						trees.get_entity(anchor)
					})
					.unwrap_or(anchor);
				let root = world.spawn_empty().id();
				RequestContextStack::scoped(
					world,
					RequestContext::new(parts, root, anchor, router)?,
					|world| source.build(&mut world.entity_mut(root)),
				)?;
				// the build's route tree wakes run while the thread is still
				// detached, so they reach its own url spaces and never the
				// surrounding one's; attached after, it resolves its ancestors
				// as if authored beneath the parent, before any seed is read
				world.flush();
				if let Some(parent) = parent {
					world.entity_mut(root).insert(ChildOf(parent));
				}
				(root, world.resource::<AsyncSpawner>().clone()).xok()
			})
			.await?;
		let function = Self {
			world: world.clone(),
			root,
			name,
			spawner,
			despawned: false,
		};
		TemplatePending::settle_root(&world.entity(root)).await;
		let error = world
			.with(move |world: &mut World| {
				world
					.get::<TemplateError>(root)
					.map(|error| error.error.clone())
			})
			.await;
		match error {
			Some(error) => {
				function.despawn().await;
				Err(error.into())
			}
			None => function.xok(),
		}
	}

	/// The build root.
	pub fn root(&self) -> Entity { self.root }

	/// The template's name.
	pub fn name(&self) -> &str { &self.name }

	/// The world it runs in.
	pub fn world(&self) -> &AsyncWorld { &self.world }

	/// Enter every thread of the build with `entry`, run the template's
	/// behavior to its [`Outcome`] and wait for every thread to reach its
	/// store, however the behavior ended.
	///
	/// ## Errors
	///
	/// Errors when the template holds a [`UserInput`], naming its actor, when
	/// it builds no single root or no `<Thread>`, when an entry is refused and
	/// when the behavior errors.
	pub async fn run(&self, entry: ThreadEntry) -> Result<ThreadOutcome> {
		let root = self.world.entity(self.root);
		let template = self.name.as_str();
		self.assert_unattended().await?;
		let threads = RunThread::enter_all(&root, &entry).await?;
		let behavior = root
			.with_state::<Query<&Children>, _>(|root, children| {
				children.get(root).ok().and_then(|children| {
					(children.len() == 1).then(|| children[0])
				})
			})
			.await?
			.ok_or_else(|| {
				bevyhow!("`{template}` must build exactly one root")
			})?;
		let thread = *threads
			.first()
			.ok_or_else(|| bevyhow!("`{template}` builds no `<Thread>`"))?;
		let outcome = self.world.entity(behavior).call::<(), Outcome>(()).await;
		RunThread::flush_all(&self.world, &threads).await?;
		ThreadOutcome {
			outcome: outcome?,
			thread,
		}
		.xok()
	}

	/// Refuse a template holding a [`UserInput`], naming its actor: nobody
	/// types into a thread run as a function. The actor is its entity's own
	/// until its thread reduces, then its window's, by [`ActorRef`].
	async fn assert_unattended(&self) -> Result {
		let template = &self.name;
		let typing = self
			.world
			.entity(self.root)
			.with_state::<(
				Query<&Children>,
				Query<(Entity, Option<&Actor>), With<UserInput>>,
				ThreadRecordsQuery,
			), _>(|root, (children, inputs, records)| {
				children
					.iter_descendants(root)
					.filter_map(|entity| inputs.get(entity).ok())
					.map(|(entity, actor)| {
						actor
							.or_else(|| records.actor(entity).ok())
							.map(|actor| actor.name().to_string())
							.unwrap_or_default()
					})
					.next()
			})
			.await?;
		match typing {
			Some(actor) => bevybail!(
				"`{template}` holds a `UserInput` on the actor `{actor}`, \
				 but a thread run as a function runs with nobody typing: a \
				 thread a person sits in is held by a `RunThread`"
			),
			None => Ok(()),
		}
	}

	/// Tear down the build, awaited.
	pub async fn despawn(mut self) {
		self.despawned = true;
		let root = self.root;
		self.world
			.with(move |world: &mut World| Self::despawn_root(world, root))
			.await;
	}

	/// Tear down the build `root`: detached from its parent first, then
	/// every entity leaves first, so each route removal records the url space
	/// it leaves while its ancestry is whole and the surrounding
	/// [`RouteTree`] never rebuilds for a thread that was never part of it.
	fn despawn_root(world: &mut World, root: Entity) {
		let Ok(mut root_mut) = world.get_entity_mut(root) else {
			return;
		};
		root_mut.remove::<ChildOf>();
		let entities = world.with_state::<Query<&Children>, _>(|children| {
			children
				.iter_descendants_inclusive(root)
				.collect::<Vec<_>>()
		});
		for entity in entities.into_iter().rev() {
			if let Ok(entity) = world.get_entity_mut(entity) {
				entity.despawn();
			}
		}
	}

	/// The last completed text an acting actor of `thread` posted, as shown:
	/// an actor with behavior beneath it ([`ActorRef`]) other than a person,
	/// so an agent's reply or a loop's decision answers, never a person's
	/// post, and never what was deleted or edited away.
	pub fn last_text(world: &mut World, thread: Entity) -> Option<String> {
		let acting = world
			.with_state::<(Query<&Children>, Query<&ActorRef>), _>(
				|(children, actor_refs)| {
					children
						.iter_descendants(thread)
						.filter_map(|entity| actor_refs.get(entity).ok())
						.map(|actor_ref| actor_ref.0.clone())
						.collect::<Vec<_>>()
				},
			);
		let records = world.get::<ThreadRecords>(thread)?;
		records
			.shown(..)
			.iter()
			.rev()
			.filter(|post| {
				acting.contains(post.author())
					&& records
						.actor(post.author())
						.is_ok_and(|actor| actor.kind() != ActorKind::User)
					&& !post.in_progress()
			})
			.find_map(|post| match post.as_agent_post()? {
				AgentPost::Text(text) => Some(text.to_string()),
				_ => None,
			})
	}
}

impl Drop for ThreadFunction {
	fn drop(&mut self) {
		if self.despawned {
			return;
		}
		let world = self.world.clone();
		let root = self.root;
		self.spawner
			.spawn(async move {
				world
					.with(move |world: &mut World| {
						Self::despawn_root(world, root)
					})
					.await;
			})
			.detach();
	}
}

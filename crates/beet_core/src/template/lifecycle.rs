//! The template build lifecycle: events, the error component, and the
//! generalized pending-dependency set that gates [`Ready`].
//!
//! The lifecycle has two observable boundaries on a template root:
//!
//! - [`SpawnTemplate`] fires once after the root's subtree is built. It is the
//!   "built" signal and the attach point for future subtree passes.
//! - [`Ready`] sweeps the loaded subtree when the root's [`TemplatePending`] set
//!   drains, immediately after [`SpawnTemplate`] when nothing is pending. It
//!   fires whether the load succeeded or failed; a failure rides
//!   [`TemplateError`] on the root.
//!
//! Slot resolution rides the same boundary: a build with no outstanding
//! [`PendingKind::Structural`] dependency resolves slots synchronously (before
//! [`SpawnTemplate`]); a build whose content is still arriving (an async
//! `<Template src>` include) defers resolution until the last structural
//! dependency resolves, so slots always match over the *settled* tree. Slots are
//! therefore guaranteed resolved by [`Ready`], but not by [`SpawnTemplate`] on a
//! deferred build.
//!
//! Build, validation, and load failures never panic: they insert
//! [`TemplateError`] on the root and the [`Ready`] sweep still fires (the
//! synchronous entrypoints also return the shared error).

use super::spawn_template::anchor_pre_slot_children;
use crate::prelude::*;
use bevy::ecs::event::SetEntityEventTarget;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;

/// Fired once on a template root after its subtree is built.
///
/// This is the post-build phase boundary: the observable hook a future subtree
/// pass attaches to without modifying the walker. For a single `spawn_template`
/// call it fires exactly once, on the root. On a build with outstanding
/// structural dependencies (an async include) slots are not yet resolved here;
/// wait for [`Ready`] to observe the settled tree.
#[derive(Debug, Clone, EntityEvent)]
pub struct SpawnTemplate {
	/// The template root.
	pub entity: Entity,
}

/// The load event: swept across a loaded subtree when the root's
/// [`TemplatePending`] dependency set drains.
///
/// One instance fires on every entity at or under the loaded root, deepest
/// first and the root last, each entity exactly once and never above the root
/// (see [`SubtreeTrigger`]), so a node observes its own readiness only after
/// everything it owns has observed theirs. When nothing is pending the sweep
/// runs synchronously, immediately after [`SpawnTemplate`]; slots are resolved
/// by the time it fires.
///
/// It fires for every load, succeeded or failed: a failure rides
/// [`TemplateError`] on the root, so a listener that cares (a load verb, an
/// awaited build) reads that off the tree rather than the event. Whether the
/// built tree *runs* is not the event's concern either: an on-ready behavior
/// (`beet_net`'s `CallOnReady`) fires by default and is opted out per subtree
/// by its own disarm marker (`DisableCallOnReady`), never by the loader
/// threading a flag through the build.
// BSN alignment: bevy's BSN ships its own `On<Ready>`, so migrating is adopting
// their trigger rather than renaming anything here.
#[derive(Debug, Clone)]
pub struct Ready {
	/// The entity this instance is firing on.
	pub entity: Entity,
}

impl Event for Ready {
	type Trigger<'a> = SubtreeTrigger<Self>;
}

impl EntityEvent for Ready {
	fn event_target(&self) -> Entity { self.entity }
}

impl SetEntityEventTarget for Ready {
	fn set_event_target(&mut self, entity: Entity) { self.entity = entity; }
}

/// Inserted on a template root whose build, validation, or load failed.
///
/// Build failures ride this path rather than panicking: the walker inserts
/// this component and the [`Ready`] sweep still fires.
#[derive(Debug, Clone, Component)]
pub struct TemplateError {
	/// The underlying error, shared (via [`CloneError`]) with the
	/// [`Ready`] event and the `spawn_template` return.
	pub error: CloneError,
}

impl TemplateError {
	/// Wraps an error for insertion on a failed root.
	pub fn new(error: impl Into<CloneError>) -> Self {
		Self {
			error: error.into(),
		}
	}
}

/// Inserted on an entry root once its declared template sources (its
/// `<TemplateDir>` directories) are registered into the
/// [`BsxTemplateRegistry`](crate::prelude::BsxTemplateRegistry) and the schemas
/// refreshed.
///
/// The build registers the entry's own template dirs *before* parsing the entry
/// (so entry-level tags like `<Styles/>` resolve), then marks the root. A driver
/// that must not serve before the entry is ready (the wasm Worker) waits for this
/// marker; the native run loop settles it naturally before the first request.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct TemplatesLoaded;

/// How a pending dependency relates to the tree it parks on.
///
/// The distinction drives deferred slot resolution: slots must wait for every
/// content-building dependency, but not for readiness-only ones (an asset that
/// gates behaviors, not structure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default)]
pub enum PendingKind {
	/// Resolution builds content into the tree (an async `<Template src>`
	/// include, a remote template). Slot resolution waits for these.
	Structural,
	/// Resolution only marks readiness (an asset load, a scene spawn, a routes
	/// scan). Gates [`Ready`], never slot resolution.
	#[default]
	Passive,
}

/// The set of outstanding dependencies gating [`Ready`] on a root.
///
/// Generalized so assets, includes, remote fetches, route scans and scene
/// spawns all register into it. Each dependency is an opaque [`PendingId`] with
/// a [`PendingKind`]. The set fires [`Ready`] when it drains to empty
/// (via [`TemplatePending::drain_dependencies`]); slot resolution deferred by the build
/// walker runs when the last [`PendingKind::Structural`] entry resolves.
///
/// A root that registers no dependencies drains immediately, so
/// [`Ready`] fires synchronously within `spawn_template`.
///
/// Prefer parking through [`TemplatePending::park`], which returns a
/// [`PendingGuard`] that cannot leak: a guard dropped unresolved (a dead task,
/// a despawned entity) resolves through the [`PendingDropQueue`] sweep, so a
/// settle can never hang on a lost dependency.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct TemplatePending {
	/// The outstanding dependencies, keyed by id.
	ids: HashMap<PendingId, PendingEntry>,
	/// The next id to hand out from [`Self::register`].
	next: u64,
	/// The pre-build [`SlotChild`] snapshot the walker parked when it deferred
	/// slot resolution to the drain (see [`TemplatePending::drain_dependencies`]).
	deferred_slots: Option<Vec<Entity>>,
}

/// An opaque identifier for one pending dependency on a [`TemplatePending`] set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct PendingId(u64);

/// One parked dependency: how it gates the load, and what it is waiting on.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
struct PendingEntry {
	kind: PendingKind,
	/// Short, specific description of the dependency, named at the park site.
	/// The only thing a bounded settle has to report when it gives up, so it
	/// identifies the *source*, eg `<RoutesDir src="routes">` or
	/// `asset "logo.png"`, not the mechanism.
	label: SmolStr,
}

/// The template root currently being built, set by the build walker for the
/// duration of a [`spawn_template`](crate::prelude::WorldTemplateExt) build.
///
/// A deferred dependency (an asset, an include, a remote schema) reads this to
/// know which entity carries the [`TemplatePending`] set its [`PendingId`] must
/// park on, so [`Ready`] defers until it resolves. Absent outside a
/// build, in which case a dependency registers on the entity it builds into.
///
/// Public so a downstream crate can build its own deferral on the same wiring
/// (eg `beet_spatial`'s scene-spawn gate parks a [`PendingGuard`] on the
/// resolved root), mirroring the asset/include deferrals.
#[derive(Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct TemplateBuildRoot(pub Entity);

impl TemplateBuildRoot {
	/// The build root recorded in `world`, falling back to `entity` when none is
	/// set (a build outside the walker), so a deferred dependency always has a
	/// root to park on.
	pub fn resolve(world: &World, entity: Entity) -> Entity {
		world
			.get_resource::<TemplateBuildRoot>()
			.map(|root| **root)
			.unwrap_or(entity)
	}

	/// Runs `func` with `root` exposed as the current build root, restoring any
	/// outer root afterwards so a nested build never leaks its root to the
	/// parent.
	///
	/// The walker wraps its build in this; an async continuation that builds
	/// content (an include resolving) wraps its `build_template` in it too, so
	/// dependencies discovered inside the continuation (a nested include) park
	/// on the *original* root rather than the include site.
	pub fn scoped<O>(
		world: &mut World,
		root: Entity,
		func: impl FnOnce(&mut World) -> O,
	) -> O {
		let previous = world.remove_resource::<TemplateBuildRoot>();
		world.insert_resource(TemplateBuildRoot(root));
		let out = func(world);
		match previous {
			Some(previous) => world.insert_resource(previous),
			None => {
				world.remove_resource::<TemplateBuildRoot>();
			}
		}
		out
	}
}

impl TemplatePending {
	/// Registers a new dependency, returning its [`PendingId`].
	///
	/// While any dependency is registered, [`Ready`] is deferred until
	/// every one is resolved via [`Self::resolve`]. Prefer [`Self::park`], whose
	/// [`PendingGuard`] cannot leak the id.
	///
	/// `label` names what is being waited on (see [`PendingEntry::label`]); it is
	/// what a bounded settle reports when it gives up, so name the source rather
	/// than the mechanism.
	pub fn register(
		&mut self,
		kind: PendingKind,
		label: impl Into<SmolStr>,
	) -> PendingId {
		let id = PendingId(self.next);
		self.next += 1;
		self.ids.insert(id, PendingEntry {
			kind,
			label: label.into(),
		});
		id
	}

	/// Resolves a previously registered dependency.
	///
	/// Returns `true` if the id was present, ie this call performed the
	/// resolution. Callers drain only on `true`, so a double resolution (a
	/// guard resolved then swept) can never re-fire [`Ready`].
	pub fn resolve(&mut self, id: PendingId) -> bool {
		self.ids.remove(&id).is_some()
	}

	/// Returns `true` if no dependencies are outstanding.
	pub fn is_empty(&self) -> bool { self.ids.is_empty() }

	/// The number of outstanding dependencies.
	pub fn len(&self) -> usize { self.ids.len() }

	/// Returns `true` if no [`PendingKind::Structural`] dependency is
	/// outstanding, ie the tree's content has settled and deferred slot
	/// resolution may run.
	pub fn structural_empty(&self) -> bool {
		!self
			.ids
			.values()
			.any(|entry| entry.kind == PendingKind::Structural)
	}

	/// Parks the walker's pre-build [`SlotChild`] snapshot, deferring slot
	/// resolution to the drain (see [`TemplatePending::drain_dependencies`]).
	pub(crate) fn defer_slots(&mut self, pre_slot_children: Vec<Entity>) {
		self.deferred_slots = Some(pre_slot_children);
	}

	/// Takes the deferred-slot snapshot, if the walker parked one.
	fn take_deferred_slots(&mut self) -> Option<Vec<Entity>> {
		self.deferred_slots.take()
	}

	/// Parks a dependency for `entity`'s build on the current build root (or on
	/// `entity` itself outside a build), returning the [`PendingGuard`] that
	/// resolves it. `label` names what is being waited on, see [`Self::register`].
	pub fn park(
		world: &mut World,
		entity: Entity,
		kind: PendingKind,
		label: impl Into<SmolStr>,
	) -> PendingGuard {
		let root = TemplateBuildRoot::resolve(world, entity);
		Self::park_on(world, root, kind, label)
	}

	/// Parks a dependency directly on `root`, returning the [`PendingGuard`]
	/// that resolves it. Prefer [`Self::park`] unless the root was already
	/// resolved (eg captured from an observer before its command ran).
	pub fn park_on(
		world: &mut World,
		root: Entity,
		kind: PendingKind,
		label: impl Into<SmolStr>,
	) -> PendingGuard {
		let id = world
			.entity_mut(root)
			.entry::<TemplatePending>()
			.or_default()
			.get_mut()
			.register(kind, label);
		let queue = world.get_resource_or_init::<PendingDropQueue>().clone();
		PendingGuard {
			root,
			id,
			queue,
			resolved: false,
		}
	}

	/// Resolves `id` on `root` and drains the set, firing [`Ready`] (and
	/// any deferred slot resolution) once nothing is outstanding. The shared
	/// tail of every resolver; a no-op if the root is gone or the id already
	/// resolved.
	pub fn resolve_on(world: &mut World, root: Entity, id: PendingId) {
		let Ok(mut root_entity) = world.get_entity_mut(root) else {
			return;
		};
		let was_present = root_entity
			.get_mut::<TemplatePending>()
			.map(|mut pending| pending.resolve(id))
			.unwrap_or(false);
		if was_present {
			TemplatePending::drain_dependencies(&mut root_entity);
		}
	}

	/// Resolves every dependency whose [`PendingGuard`] was dropped unresolved
	/// (a dead task, a despawned holder), draining the affected roots.
	///
	/// Ran by the [`TemplatePlugin`](crate::prelude::TemplatePlugin) each frame
	/// and by every settle pass, so a lost dependency delays a settle by at most
	/// one sync point rather than hanging it.
	pub fn sweep_dropped(world: &mut World) {
		let Some(queue) = world.get_resource::<PendingDropQueue>() else {
			return;
		};
		let dropped = queue.take();
		for (root, id) in dropped {
			Self::resolve_on(world, root, id);
		}
	}

	/// Every dependency still outstanding anywhere in the world, as
	/// `(root, label, kind)` triples: what a settle is actually waiting on.
	fn outstanding(world: &mut World) -> Vec<(Entity, SmolStr, PendingKind)> {
		world
			.query::<(Entity, &TemplatePending)>()
			.iter(world)
			.flat_map(|(root, pending)| {
				pending
					.ids
					.values()
					.map(move |entry| (root, entry.label.clone(), entry.kind))
			})
			.collect()
	}

	/// The message a bounded settle fails with: every outstanding dependency
	/// named alongside the entity carrying it, so the report identifies what
	/// wedged rather than just how many did.
	fn timeout_report(
		deadline: Duration,
		outstanding: &[(Entity, SmolStr, PendingKind)],
	) -> String {
		let plural = if outstanding.len() == 1 { "y" } else { "ies" };
		let lines = outstanding
			.iter()
			.map(|(root, label, kind)| {
				format!("\n  - {label} ({kind:?} dependency of {root})")
			})
			.collect::<String>();
		format!(
			"timed out after {deadline:?} waiting on {} unresolved template dependenc{plural}:{lines}",
			outstanding.len(),
		)
	}
}

/// Dependencies whose [`PendingGuard`] was dropped unresolved, awaiting the
/// [`TemplatePending::sweep_dropped`] sweep.
///
/// Arc-shared with every guard so a `Drop` impl (which cannot reach the world)
/// still lands its resolution; a clone of this resource outlives the world
/// harmlessly.
#[derive(Debug, Default, Clone, Resource)]
pub struct PendingDropQueue(Arc<Mutex<Vec<(Entity, PendingId)>>>);

impl PendingDropQueue {
	/// Push a dropped dependency for the next sweep.
	fn push(&self, root: Entity, id: PendingId) {
		if let Ok(mut queue) = self.0.lock() {
			queue.push((root, id));
		}
	}

	/// Take every queued dropped dependency.
	fn take(&self) -> Vec<(Entity, PendingId)> {
		self.0
			.lock()
			.map(|mut queue| core::mem::take(&mut *queue))
			.unwrap_or_default()
	}

	/// Whether any dropped dependency awaits a sweep.
	pub fn is_empty(&self) -> bool {
		self.0.lock().map(|queue| queue.is_empty()).unwrap_or(true)
	}
}

/// Owned handle to one parked dependency on a [`TemplatePending`] set.
///
/// Resolving drains the set (firing [`Ready`] and any deferred slot
/// resolution once nothing is outstanding). Dropping it unresolved queues the
/// resolution onto the [`PendingDropQueue`] sweep instead, so a dependency can
/// never leak and hang a settle: an async task holds its guard across the
/// fetch, and whether it resolves, errors, panics, or is dropped mid-flight,
/// the set drains.
#[derive(Debug)]
pub struct PendingGuard {
	root: Entity,
	id: PendingId,
	queue: PendingDropQueue,
	resolved: bool,
}

impl PendingGuard {
	/// The root carrying the [`TemplatePending`] set this guard parked on.
	pub fn root(&self) -> Entity { self.root }

	/// Resolves the dependency and drains the root's set.
	pub fn resolve(mut self, world: &mut World) {
		self.resolved = true;
		TemplatePending::resolve_on(world, self.root, self.id);
	}
}

impl Drop for PendingGuard {
	fn drop(&mut self) {
		if self.resolved {
			return;
		}
		// `debug!`, not `warn!`: this is the routine teardown path as much as the
		// failure path. Tearing a scene down (every structural live reload) drops
		// the tracker holding any in-flight dependency, so a warn here would be
		// noise on a healthy edit loop. The sweep resolves either way.
		debug!(
			"pending template dependency on {} dropped unresolved; resolving via sweep",
			self.root
		);
		self.queue.push(self.root, self.id);
	}
}

/// A [`PendingGuard`] carried by a component: removing the component (or
/// despawning its entity) resolves the dependency.
///
/// The entity-lifetime counterpart of holding a guard in a task: a deferral
/// keyed to an entity's readiness (eg a spawning world scene) inserts this and
/// simply removes it when ready; a teardown that despawns the entity resolves
/// it implicitly, so the root can never hang on a vanished dependent.
#[derive(Debug, Component)]
#[component(on_remove = PendingDependency::on_remove())]
pub struct PendingDependency(PendingGuard);

impl PendingDependency {
	/// Wrap a parked guard for entity-lifetime resolution.
	pub fn new(guard: PendingGuard) -> Self { Self(guard) }

	/// The `on_remove` hook: defuse the guard's drop path and queue the
	/// resolution, so removal and despawn both resolve exactly once.
	fn on_remove() -> impl FnOnce(
		bevy::ecs::world::DeferredWorld,
		bevy::ecs::lifecycle::HookContext,
	) {
		|mut world, cx| {
			let Some(mut dep) = world.get_mut::<PendingDependency>(cx.entity)
			else {
				return;
			};
			dep.0.resolved = true;
			let (root, id) = (dep.0.root, dep.0.id);
			world.commands().queue(move |world: &mut World| {
				TemplatePending::resolve_on(world, root, id);
			});
		}
	}
}

impl TemplatePending {
	/// Fires [`Ready`] on `root` if its [`TemplatePending`] set is empty (or
	/// absent), reporting the error state from the presence of [`TemplateError`].
	///
	/// This is the drain trigger, called synchronously by the walker after
	/// [`SpawnTemplate`] (the empty case) and by every dependency resolver via
	/// [`TemplatePending::resolve_on`]. Two things happen here:
	///
	/// 1. Once no [`PendingKind::Structural`] dependency remains, a deferred slot
	///    resolution (parked by the walker when content was still arriving) runs
	///    exactly once over the settled tree; a failure rides [`TemplateError`].
	/// 2. Once nothing at all remains, the [`Ready`] sweep runs over the loaded
	///    subtree.
	pub fn drain_dependencies(root: &mut EntityWorldMut) {
		// deferred slot resolution: the content has settled once every structural
		// dependency resolved. `take_deferred_slots` yields at most once.
		let structural_empty = root
			.get::<TemplatePending>()
			.map(TemplatePending::structural_empty)
			.unwrap_or(true);
		if structural_empty
			&& let Some(pre_slot_children) = root
				.get_mut::<TemplatePending>()
				.and_then(|mut pending| pending.take_deferred_slots())
			&& !root.contains::<TemplateError>()
		{
			let root_id = root.id();
			let result = root.world_scope(|world| {
				anchor_pre_slot_children(world, root_id, &pre_slot_children);
				resolve_slots(world, root_id)
			});
			if let Err(err) = result {
				root.insert(TemplateError::new(CloneError::new(err)));
			}
		}

		let pending_empty = root
			.get::<TemplatePending>()
			.map(TemplatePending::is_empty)
			.unwrap_or(true);
		if !pending_empty {
			return;
		}
		// the sweep reaches the root *and* every descendant in the built subtree, so
		// an on-ready listener (eg `CallOnReady`) sitting on any node observes its
		// own `Ready` locally, deepest first.
		root.trigger_subtree(|entity| Ready { entity });
	}
}

/// The system form of [`TemplatePending::sweep_dropped`], registered by the
/// [`TemplatePlugin`](crate::prelude::TemplatePlugin) so a running app resolves
/// dropped guards every frame.
pub(crate) fn sweep_dropped_pending(world: &mut World) {
	TemplatePending::sweep_dropped(world);
}

// both settles pace themselves with `async_ext::yield_now`, which rides
// `futures_lite`: std-only, like the `settle_owned` block below.
#[cfg(all(feature = "bevy_async", feature = "std"))]
impl TemplatePending {
	/// Awaits quiescence of every [`TemplatePending`] set in the world, the
	/// in-app settle a caller on the async runtime uses (an action rendering
	/// routes it just built, the live-reload driver).
	///
	/// Sweeps dropped guards each pass, so a lost dependency cannot hang it;
	/// there is no arbitrary deadline, but a long wait warns periodically so a
	/// genuinely stuck dependency (a hung store read) is observable.
	pub async fn settle(world: &AsyncWorld) {
		let mut last_warn = Instant::now();
		loop {
			let pending = world
				.with(|world| {
					// apply queued registrations so a just-inserted dependency counts.
					world.flush();
					Self::sweep_dropped(world);
					world
						.query::<&TemplatePending>()
						.iter(world)
						.filter(|pending| !pending.is_empty())
						.count()
				})
				.await;
			if pending == 0 {
				return;
			}
			if last_warn.elapsed() > Duration::from_secs(5) {
				warn!(
					"still waiting on {pending} pending template dependency set(s)"
				);
				last_warn = Instant::now();
			}
			// Yield, then take an extra bridge round-trip before the next count.
			// The dependency tasks bridge the world between async store reads; a
			// single yield + count can lap them, re-reading the same counts while
			// their just-woken bridge poll still waits for a sync-point window. The
			// no-op round-trip drives one more sync point so a completed read makes
			// progress before the re-count.
			async_ext::yield_now().await;
			world.with(|_| ()).await;
		}
	}

	/// [`Self::settle`] with a ceiling: fails once `deadline` elapses with a
	/// dependency still outstanding.
	///
	/// The bounded settle a one-shot command uses (`check`, `export-static`,
	/// `export-pdf`): those produce a result and exit, so a dependency that never
	/// resolves has to fail the command rather than hang it. A long-running app
	/// (`serve` included) settles through [`Self::settle`] instead, where a slow
	/// dependency is a stall to wait out, not a failed run.
	///
	/// The error names every outstanding dependency and its entity, so a wedged
	/// build reports which dependency wedged.
	pub async fn settle_before(
		world: &AsyncWorld,
		deadline: Duration,
	) -> Result {
		let started = Instant::now();
		loop {
			let outstanding = world
				.with(|world| {
					// apply queued registrations so a just-inserted dependency counts.
					world.flush();
					Self::sweep_dropped(world);
					Self::outstanding(world)
				})
				.await;
			if outstanding.is_empty() {
				return Ok(());
			}
			if started.elapsed() > deadline {
				bevybail!("{}", Self::timeout_report(deadline, &outstanding));
			}
			// same two-step yield as `settle`: a single yield can lap the
			// dependency tasks' bridge polls, see its comment.
			async_ext::yield_now().await;
			world.with(|_| ()).await;
		}
	}
}

// the world-owning settle drives [`AsyncRunner`], which needs a task pool: std-only.
#[cfg(all(feature = "bevy_async", feature = "std"))]
impl TemplatePending {
	/// The world-owning settle: drives the async runtime itself until every
	/// [`TemplatePending`] set drains, for a build-then-serve driver that owns
	/// the world (the wasm Worker, a one-shot build) rather than running inside
	/// the app loop. Warns periodically like [`Self::settle`].
	pub async fn settle_owned(world: &mut World) {
		let mut last_warn = Instant::now();
		loop {
			AsyncRunner::settle_async_tasks(world).await;
			world.flush();
			Self::sweep_dropped(world);
			let pending = world
				.query::<&TemplatePending>()
				.iter(world)
				.filter(|pending| !pending.is_empty())
				.count();
			if pending == 0 {
				return;
			}
			if last_warn.elapsed() > Duration::from_secs(5) {
				warn!(
					"still waiting on {pending} pending template dependency set(s)"
				);
				last_warn = Instant::now();
			}
			AsyncRunner::tick().await;
		}
	}
}

// the bounded settle rides `bevy_async`, and driving it needs a task pool: std-only.
#[cfg(all(test, feature = "bevy_async", feature = "std"))]
mod test {
	use crate::prelude::*;

	/// A bounded settle fails rather than hanging on a dependency that never
	/// resolves, and its error names that dependency and the entity carrying it,
	/// so a wedged one-shot build reports *what* it waited on.
	#[crate::test]
	async fn settle_before_names_the_stuck_dependency() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, TemplatePlugin));
		let root = app.world_mut().spawn_empty().id();
		// registered without a guard, so it can neither resolve nor be swept: the
		// dependency a bounded settle exists to report.
		app.world_mut()
			.entity_mut(root)
			.entry::<TemplatePending>()
			.or_default()
			.get_mut()
			.register(PendingKind::Passive, "<RoutesDir src=\"routes\">");

		let err = app
			.world_mut()
			.run_async_local_then(|world| async move {
				TemplatePending::settle_before(
					&world,
					Duration::from_millis(50),
				)
				.await
			})
			.await
			.unwrap_err()
			.to_string();
		err.clone().xpect_contains("<RoutesDir src=\"routes\">");
		err.xpect_contains(root.to_string());
	}

	/// With nothing outstanding the bounded settle returns immediately, ie the
	/// deadline only bites on a real stall.
	#[crate::test]
	async fn settle_before_passes_when_nothing_is_pending() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, TemplatePlugin));
		app.world_mut()
			.run_async_local_then(|world| async move {
				TemplatePending::settle_before(&world, Duration::from_secs(5))
					.await
			})
			.await
			.unwrap();
	}
}

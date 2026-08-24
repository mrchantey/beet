//! Remote schemas and remote templates: async front-ends that defer
//! [`LoadTemplate`](beet_core::prelude::LoadTemplate) until they resolve.
//!
//! Both are async because a remote dependency forms a graph that resolves over
//! the network. They park a [`PendingGuard`](beet_core::prelude::PendingGuard)
//! on the build root's [`TemplatePending`](beet_core::prelude::TemplatePending)
//! set, spawn a task that resolves the dependency, then resolve the guard,
//! firing `LoadTemplate` once everything settles.
//!
//! The fetch itself is intentionally unimplemented: remote transport (trust,
//! caching, versioning) is deferred to the BSX to BSN transition, which
//! replaces this front-end. What is real is the pending-set wiring, which local
//! includes (`beet_router`'s `<Template src>`) already run on.
//!
//! Gated behind `bevy_async`: the no_std core never references this.

use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Register a pending remote-schema fetch on the build root, so `LoadTemplate`
/// defers until the schema resolves.
///
/// Parks a [`PendingGuard`] on the root's [`TemplatePending`], then spawns a
/// task that fetches the schema at `url` (stubbed), registers it in the
/// [`SchemaRegistry`] under `name`, and resolves the guard.
///
/// The async resolution + validation is therefore registered into the
/// `LoadTemplate` pending set, exactly as assets are.
pub(crate) fn register_remote_schema(
	name: SmolStr,
	url: SmolStr,
	cx: &mut TemplateContext,
) -> Result {
	let entity_id = cx.entity.id();
	// SAFETY: only used to register the pending dependency and read the spawner.
	let world = unsafe { cx.entity.world_mut() };
	// a schema gates validation, not tree content: passive.
	let (async_world, spawner, guard) = TemplatePending::register_fetch(
		world,
		entity_id,
		PendingKind::Passive,
		format!("remote schema `{name}` at `{url}`"),
	)?;
	spawner.spawn(resolve_remote_schema(async_world, name, url, guard));
	Ok(())
}

impl TemplatePending {
	/// Park a [`PendingGuard`] on the build root's pending set and read the async
	/// runtime handles, erroring gracefully if the async runtime is absent.
	///
	/// Returns the [`AsyncWorld`] + [`AsyncSpawner`] to drive the fetch and the
	/// guard to resolve once the dependency lands; a guard dropped by a dead task
	/// resolves through the sweep, so the fetch can never hang the load. Public so
	/// a higher layer can build its own store-backed front-end on the same wiring
	/// (eg `beet_router`'s `<Template src>` include reads the bytes through a
	/// `BlobStore` it alone can reference).
	pub fn register_fetch(
		world: &mut World,
		entity: Entity,
		kind: PendingKind,
		label: impl Into<SmolStr>,
	) -> Result<(AsyncWorld, AsyncSpawner, PendingGuard)> {
		let (Some(async_world), Some(spawner)) = (
			world.get_resource::<AsyncWorld>().cloned(),
			world.get_resource::<AsyncSpawner>().cloned(),
		) else {
			bevybail!(
				"a remote schema/template needs the async runtime (add `AsyncPlugin`)"
			);
		};
		let guard = TemplatePending::park(world, entity, kind, label);
		Ok((async_world, spawner, guard))
	}
}

/// Fetch (stubbed), register, then resolve a remote schema's pending dependency.
async fn resolve_remote_schema(
	async_world: AsyncWorld,
	name: SmolStr,
	url: SmolStr,
	guard: PendingGuard,
) {
	// no transport (see the module doc): resolves to an unconstrained schema so
	// the wiring is live.
	let schema = fetch_remote_schema(&url).await;

	async_world
		.with(move |world: &mut World| {
			world
				.get_resource_or_init::<SchemaRegistry>()
				.insert(name, schema);
			guard.resolve(world);
		})
		.await;
}

/// Stubbed remote-schema fetch: resolves to [`ValueSchema::Any`] (a wildcard),
/// so a remote schema validates everything. A real transport is deferred to the
/// BSN transition; this signature is the seam it would drop into.
async fn fetch_remote_schema(_url: &str) -> ValueSchema { ValueSchema::Any }

/// Register a pending remote-template fetch on the build root for a
/// `<Template src="..">` tag, deferring `LoadTemplate` until it resolves.
///
/// A remote template is another front-end producing a
/// [`DynamicTemplate`](beet_core::prelude::DynamicTemplate), fetched
/// asynchronously and resolved through the same registry as `<path::to::X>`.
/// This is the stub: it parks a [`PendingGuard`] and spawns a task that resolves
/// it, so a real fetch slots in later without rework.
pub(crate) fn register_remote_template(
	src: SmolStr,
	cx: &mut TemplateContext,
) -> Result {
	let entity_id = cx.entity.id();
	// SAFETY: only used to register the pending dependency and read the spawner.
	let world = unsafe { cx.entity.world_mut() };
	// a remote template builds content at the include site: structural.
	let (async_world, spawner, guard) = TemplatePending::register_fetch(
		world,
		entity_id,
		PendingKind::Structural,
		format!("remote template `{src}`"),
	)?;
	spawner.spawn(resolve_remote_template(async_world, src, entity_id, guard));
	Ok(())
}

/// Fetch (stubbed), build, then resolve a remote template's pending dependency.
async fn resolve_remote_template(
	async_world: AsyncWorld,
	src: SmolStr,
	target: Entity,
	guard: PendingGuard,
) {
	// no transport (see the module doc): the include site stays empty. Fetching
	// and building the remote `.bsx` is deferred to the BSN transition.
	let _ = (&src, target);

	async_world
		.with(move |world: &mut World| {
			guard.resolve(world);
		})
		.await;
}

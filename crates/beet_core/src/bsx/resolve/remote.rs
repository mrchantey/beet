//! Remote schemas and remote templates: async front-ends that defer
//! [`Ready`](beet_core::prelude::Ready) until they resolve.
//!
//! Both are async because a remote dependency forms a graph that resolves over
//! the network. They park a [`PendingGuard`](beet_core::prelude::PendingGuard)
//! on the build root's [`TemplatePending`](beet_core::prelude::TemplatePending)
//! set, spawn a task that resolves the dependency, then resolve the guard,
//! firing `Ready` once everything settles.
//!
//! The fetch itself is intentionally unimplemented: remote transport (trust,
//! caching, versioning) is deferred to the BSX to BSN transition, which
//! replaces this front-end. What is real is the pending-set wiring, which local
//! includes (`beet_router`'s `<Template src>`) already run on.
//!
//! Gated behind `bevy_async`: the no_std core never references this.

use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Register a pending remote-schema fetch on the build root, so `Ready`
/// defers until the schema resolves.
///
/// Parks a [`PendingGuard`] on the root's [`TemplatePending`], then spawns a
/// task that fetches the schema at `url` (stubbed), registers it in the
/// [`SchemaRegistry`] under `name`, and resolves the guard.
///
/// The async resolution + validation is therefore registered into the
/// `Ready` pending set, exactly as assets are.
pub(super) fn register_remote_schema(
	name: SmolStr,
	url: SmolStr,
	cx: &mut TemplateContext,
) -> Result {
	// a schema gates validation, not tree content: passive.
	TemplatePending::register_fetch(
		cx.entity,
		PendingKind::Passive,
		format!("remote schema `{name}` at `{url}`"),
		move |entity, guard| resolve_remote_schema(entity, name, url, guard),
	)
}

impl TemplatePending {
	/// Park a [`PendingGuard`] on the build root's pending set and spawn `fetch`
	/// as a task of `entity`, handing it the guard to resolve once the
	/// dependency lands. Errors gracefully if the async runtime is absent.
	///
	/// The task ends with the entity, and a guard dropped by a cancelled or dead
	/// task resolves through the sweep, so the fetch can never hang the load.
	/// Public so a higher layer can build its own store-backed front-end on the
	/// same wiring (eg `beet_router`'s `<Template src>` include reads the bytes
	/// through a `BlobStore` it alone can reference).
	pub fn register_fetch<Func, Fut>(
		entity: &mut EntityWorldMut,
		kind: PendingKind,
		label: impl Into<SmolStr>,
		fetch: Func,
	) -> Result
	where
		Func: 'static + Send + FnOnce(AsyncEntity, PendingGuard) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = ()>,
	{
		let id = entity.id();
		let guard = entity.world_scope(|world| -> Result<PendingGuard> {
			if !world.contains_resource::<AsyncWorld>()
				|| !world.contains_resource::<AsyncSpawner>()
			{
				bevybail!(
					"a remote schema/template needs the async runtime (add `AsyncPlugin`)"
				);
			}
			TemplatePending::park(world, id, kind, label).xok()
		})?;
		entity.run_async(move |entity| fetch(entity, guard));
		Ok(())
	}
}

/// Fetch (stubbed), register, then resolve a remote schema's pending dependency.
async fn resolve_remote_schema(
	entity: AsyncEntity,
	name: SmolStr,
	url: SmolStr,
	guard: PendingGuard,
) {
	// no transport (see the module doc): resolves to an unconstrained schema so
	// the wiring is live.
	let schema = fetch_remote_schema(&url).await;

	entity
		.world()
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
/// `<Template src="..">` tag, deferring `Ready` until it resolves.
///
/// A remote template is another front-end producing a
/// [`DynamicTemplate`](beet_core::prelude::DynamicTemplate), fetched
/// asynchronously and resolved through the same registry as `<path::to::X>`.
/// This is the stub: it parks a [`PendingGuard`] and spawns a task that resolves
/// it, so a real fetch slots in later without rework.
pub(super) fn register_remote_template(
	src: SmolStr,
	cx: &mut TemplateContext,
) -> Result {
	// a remote template builds content at the include site: structural.
	TemplatePending::register_fetch(
		cx.entity,
		PendingKind::Structural,
		format!("remote template `{src}`"),
		move |entity, guard| resolve_remote_template(entity, src, guard),
	)
}

/// Fetch (stubbed), build, then resolve a remote template's pending dependency.
async fn resolve_remote_template(
	entity: AsyncEntity,
	src: SmolStr,
	guard: PendingGuard,
) {
	// no transport (see the module doc): the include site stays empty. Fetching
	// and building the remote `.bsx` is deferred to the BSN transition.
	let _ = &src;

	entity
		.world()
		.with(move |world: &mut World| {
			guard.resolve(world);
		})
		.await;
}

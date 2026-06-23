//! Remote schemas and remote templates: async front-ends that defer
//! [`LoadTemplate`](beet_core::prelude::LoadTemplate) until they resolve.
//!
//! Both are async because a remote dependency forms a graph that resolves over
//! the network. They register a [`PendingId`](beet_core::prelude::PendingId) into
//! the build root's [`TemplatePending`](beet_core::prelude::TemplatePending) set,
//! spawn a task that resolves the dependency, then resolve the id and drain the
//! set, firing `LoadTemplate` once everything settles.
//!
//! The fetch itself is STUBBED: trust, caching, versioning, and the actual
//! transport are deliberate later decisions (see the TODOs). What is real is the
//! pending-set wiring, so the transport slots in later without rework.
//!
//! Gated behind `bevy_async`: the no_std core never references this.

use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Register a pending remote-schema fetch on the build root, so `LoadTemplate`
/// defers until the schema resolves.
///
/// Parks a [`PendingId`] on the root's [`TemplatePending`], then spawns a task
/// that fetches the schema at `url` (stubbed), registers it in the
/// [`SchemaRegistry`] under `name`, resolves the id, and drains the pending set.
///
/// The async resolution + validation is therefore registered into the
/// `LoadTemplate` pending set, exactly as assets are.
pub fn register_remote_schema(
	name: SmolStr,
	url: SmolStr,
	cx: &mut TemplateContext,
) -> Result {
	let entity_id = cx.entity.id();
	// SAFETY: only used to register the pending dependency and read the spawner.
	let world = unsafe { cx.entity.world_mut() };
	let (async_world, spawner, root, pending_id) =
		register_pending_fetch(world, entity_id)?;
	spawner.spawn(resolve_remote_schema(
		async_world,
		name,
		url,
		root,
		pending_id,
	));
	Ok(())
}

/// Park a [`PendingId`] on the build root's pending set and read the async
/// runtime handles, erroring gracefully if the async runtime is absent.
///
/// Returns the [`AsyncWorld`] + [`AsyncSpawner`] to drive the fetch, the build
/// `root` carrying the pending set, and the [`PendingId`] to resolve once the
/// dependency lands. Public so a higher layer can build its own store-backed
/// front-end on the same wiring (eg `beet_router`'s `<Template src>` include reads
/// the bytes through a `BlobStore` it alone can reference).
pub fn register_pending_fetch(
	world: &mut World,
	entity: Entity,
) -> Result<(AsyncWorld, AsyncSpawner, Entity, PendingId)> {
	let (Some(async_world), Some(spawner)) = (
		world.get_resource::<AsyncWorld>().cloned(),
		world.get_resource::<AsyncSpawner>().cloned(),
	) else {
		bevybail!(
			"a remote schema/template needs the async runtime (add `AsyncPlugin`)"
		);
	};
	let root = TemplateBuildRoot::resolve(world, entity);
	let pending_id = world
		.entity_mut(root)
		.entry::<TemplatePending>()
		.or_default()
		.get_mut()
		.register();
	Ok((async_world, spawner, root, pending_id))
}

/// Fetch (stubbed), register, then resolve a remote schema's pending dependency.
async fn resolve_remote_schema(
	async_world: AsyncWorld,
	name: SmolStr,
	url: SmolStr,
	root: Entity,
	pending_id: PendingId,
) {
	// TODO: actually fetch `url` over the network with trust + caching + versioning.
	// For now the stub resolves to an unconstrained schema so the wiring is live.
	let _ = &url;
	let schema = fetch_remote_schema(&url).await;

	async_world
		.with(move |world: &mut World| {
			world
				.get_resource_or_init::<SchemaRegistry>()
				.insert(name, schema);
			let mut root_entity = world.entity_mut(root);
			if let Some(mut pending) = root_entity.get_mut::<TemplatePending>()
			{
				pending.resolve(pending_id);
			}
			drain_pending_dependencies(&mut root_entity);
		})
		.await;
}

/// Stubbed remote-schema fetch: resolves to [`ValueSchema::Any`] (a wildcard).
///
/// TODO: fetch and deserialize the JSON schema at `url`, with trust, caching, and
/// versioning. The signature is the seam the real transport drops into.
async fn fetch_remote_schema(_url: &str) -> ValueSchema { ValueSchema::Any }

/// Register a pending remote-template fetch on the build root for a
/// `<Template src="..">` tag, deferring `LoadTemplate` until it resolves.
///
/// A remote template is another front-end producing a
/// [`DynamicTemplate`](beet_core::prelude::DynamicTemplate), fetched
/// asynchronously and resolved through the same registry as `<path::to::X>`.
/// This is the stub: it parks a [`PendingId`] and spawns a task that resolves it,
/// so a real fetch slots in later without rework.
pub fn register_remote_template(
	src: SmolStr,
	cx: &mut TemplateContext,
) -> Result {
	let entity_id = cx.entity.id();
	// SAFETY: only used to register the pending dependency and read the spawner.
	let world = unsafe { cx.entity.world_mut() };
	let (async_world, spawner, root, pending_id) =
		register_pending_fetch(world, entity_id)?;
	spawner.spawn(resolve_remote_template(
		async_world,
		src,
		entity_id,
		root,
		pending_id,
	));
	Ok(())
}

/// Fetch (stubbed), build, then resolve a remote template's pending dependency.
async fn resolve_remote_template(
	async_world: AsyncWorld,
	src: SmolStr,
	target: Entity,
	root: Entity,
	pending_id: PendingId,
) {
	// TODO: fetch the `.bsx` (or serialized `DynamicTemplate`) at `src`, parse it
	// to a `DynamicTemplate`, then `build_template` it into `target` through the
	// same registry as `<path::to::X>`. Trust, caching, and versioning are later.
	let _ = (&src, target);

	async_world
		.with(move |world: &mut World| {
			let mut root_entity = world.entity_mut(root);
			if let Some(mut pending) = root_entity.get_mut::<TemplatePending>()
			{
				pending.resolve(pending_id);
			}
			drain_pending_dependencies(&mut root_entity);
		})
		.await;
}

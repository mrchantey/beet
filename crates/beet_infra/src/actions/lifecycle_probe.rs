//! Full-lifecycle smoke-test action for an infra stack.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Exercises a stack's full lifecycle for demos/smoke-tests: force-reset, validate,
/// plan, apply, write a probe file to the bucket's [`BlobStore`] and read it back,
/// then destroy — logging each step. Resolves the [`Stack`] via [`StackQuery`]; reads
/// the [`BlobStore`] from its own entity, so spawn it on the bucket block entity (eg
/// `<NamedBucket label="my-bucket" {LifecycleProbe}/>`).
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn LifecycleProbe(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// resolve the project (config of every block descendant) + the bucket store
	// from this entity in one pass.
	let (project, store) = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| -> Result<_> {
			let project = query.build_project(entity)?;
			let store = query.store(entity)?.clone();
			(project, store).xok()
		})
		.await??;

	// reset state in case of a backend change, clearing any stale store.
	project.force_destroy().await;
	if store.store_exists().await.unwrap_or(false) {
		info!("🧹 Cleaning up stale store..");
		store.store_remove().await.ok();
	}

	info!("🔨 Validating..");
	project.validate().await?;

	info!("🔨 Planning..");
	project.plan().await?;

	// state file and bucket dont exist yet, we are pre-apply.
	info!(
		"📦 State file exists: {}",
		project.state_file().exists().await?
	);
	info!("🪣 BlobStore Exists: {}", store.store_exists().await?);

	info!("🔨 Applying..");
	project.apply().await?;

	info!(
		"📦 State File exists: {}",
		project.state_file().exists().await?
	);
	info!("🪣 BlobStore Exists: {}", store.store_exists().await?);

	let path = SmolPath::new("foo.md");
	let content = "bar";
	info!(
		"📄 BlobStore File Exists: {}",
		store.get(&path).await.is_ok()
	);

	info!("🔨 Inserting File..");
	store.insert(&path, content).await?;
	let bytes = store.get(&path).await?;
	info!("📄 BlobStore File Matches: {}", bytes == content.as_bytes());

	info!("🔨 Destroying..");
	project.destroy().await?;

	info!(
		"📦 State file exists: {}",
		project.state_file().exists().await?
	);
	info!("🪣 BlobStore Exists: {}", store.store_exists().await?);

	Pass(cx.input).xok()
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	// the probe reads its store from its OWN entity, so confirm `StackQuery::store`
	// resolves a `BlobStore` on the queried entity (the `{LifecycleProbe}` spread
	// layout), not only from a sibling.
	#[beet_core::test]
	fn store_resolves_from_own_entity() {
		let (stack, _dir) = Stack::default_local();
		let mut world = World::new();
		let entity = world.spawn((stack, BlobStore::temp())).id();
		world.with_state::<StackQuery, _>(|query| {
			query.store(entity).is_ok().xpect_true();
		});
	}
}

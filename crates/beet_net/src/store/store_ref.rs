//! The relationship binding a store *consumer* to a declared store.

use beet_core::prelude::*;

/// The entity whose store this consumer reads and writes: the source half of the
/// [`StoreConsumers`] relationship.
///
/// Backend-agnostic on purpose. A provider component materializes both erased
/// currencies on its entity (a [`BlobStore`](crate::prelude::BlobStore) and a
/// [`TableStore`](crate::prelude::TableStore)), so the consumer picks whichever
/// it reads off the target rather than the relation naming one.
///
/// A resource is declared once, as its provider block, and every consumer names
/// that declaration rather than a backend or a resource name. Spawn it beside the
/// consumer marker so the relationship machinery remaps its `$name` reference:
///
/// ```html
/// <DynamoTableBlock bx:ref="analytics" label="analytics"/>
/// <Router {(AnalyticsConfig, StoreRef($analytics))}>..</Router>
/// ```
///
/// `allow_self_referential` so a consumer co-located with its store on one
/// entity still links.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = StoreConsumers, allow_self_referential)]
pub struct StoreRef(#[entities] pub Entity);

impl StoreRef {
	/// The store entity this consumer is bound to.
	pub fn store(&self) -> Entity { self.0 }

	/// Read the erased store component a declaration entity materializes, ie a
	/// [`TableStore`](crate::prelude::TableStore) or a
	/// [`BlobStore`](crate::prelude::BlobStore).
	///
	/// A declaration's runtime half lands through the command queue (the
	/// ancestry a stack scope resolves against arrives with the rest of the
	/// scene), so a consumer reading one straight off the entity would race it.
	/// Backs off before failing, and the failure names the entity so an operator
	/// can see which declaration answered nothing.
	pub async fn resolve<T: Component + Clone>(
		world: &AsyncWorld,
		target: Entity,
	) -> Result<T> {
		let mut backoff = Backoff::default().with_max_attempts(5).stream();
		loop {
			if let Ok(store) = world
				.entity(target)
				.get::<T, _>(|store| store.clone())
				.await
			{
				return Ok(store);
			}
			if backoff.next().await.is_none() {
				bevybail!(
					"store entity {target} has no `{}`: give the declaration a store \
					 provider component, ie `<FsStore/>`",
					core::any::type_name::<T>()
				);
			}
		}
	}
}

/// Every consumer bound to a store: the target half of the [`StoreRef`]
/// relationship, on the store entity.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = StoreRef)]
pub struct StoreConsumers(Vec<Entity>);

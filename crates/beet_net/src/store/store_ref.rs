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
}

/// Every consumer bound to a store: the target half of the [`StoreRef`]
/// relationship, on the store entity.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = StoreRef)]
pub struct StoreConsumers(Vec<Entity>);

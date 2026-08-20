//! The relationship binding a store *consumer* to a declared store.

use beet_core::prelude::*;

/// The entity whose [`TableStore`](crate::prelude::TableStore) this consumer
/// reads and writes: the source half of the [`TableStoreConsumers`]
/// relationship.
///
/// A resource is declared once, as its provider block, and every consumer names
/// that declaration rather than a backend or a table name. Spawn it beside the
/// consumer marker so the relationship machinery remaps its `$name` reference:
///
/// ```html
/// <DynamoTableBlock bx:ref="analytics" label="analytics"/>
/// <Router {(AnalyticsConfig, TableStoreRef($analytics))}>..</Router>
/// ```
///
/// `allow_self_referential` so a consumer co-located with its store on one
/// entity still links.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = TableStoreConsumers, allow_self_referential)]
pub struct TableStoreRef(#[entities] pub Entity);

impl TableStoreRef {
	/// The store entity this consumer is bound to.
	pub fn store(&self) -> Entity { self.0 }
}

/// Every consumer bound to a store: the target half of the [`TableStoreRef`]
/// relationship, on the store entity.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = TableStoreRef)]
pub struct TableStoreConsumers(Vec<Entity>);

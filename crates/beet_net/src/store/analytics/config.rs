//! The [`AnalyticsConfig`] on-switch component.
use beet_core::prelude::*;

/// Per-router analytics settings, and the on-switch: spawning it resolves the
/// store, so a beet app records nothing until one is present.
///
/// A component on the router entity, authorable from markup as
/// `<AnalyticsConfig/>`. Terminal page views and web beacons persist once it
/// exists; the fields tune the [`AnalyticsKind::Request`](super::AnalyticsKind)
/// stream the router middleware records.
#[derive(Debug, Clone, Component, Reflect, MapEntities)]
#[reflect(Component, Default, MapEntities)]
pub struct AnalyticsConfig {
	/// Record a request event per routed request. On by default; set `false` to
	/// keep only the client-reported streams.
	pub record_requests: bool,
	/// Store the raw client ip on events. Off by default, so the default posture
	/// derives only a country and collects no personal data.
	pub store_ip: bool,
	/// The entity carrying the analytics `TableStore`: any store provider
	/// component materializes one, so an entity spawned with ie `<FsStore/>` or
	/// `<DynamoStore/>` qualifies, referenced from markup as `store=$my_store`
	/// (a `bx:ref` name). `None` derives the store by convention, spawning a
	/// child store entity: a local run writes `WorkspaceConfig::analytics_dir`,
	/// a deployed (`ServiceAccess::Remote`) process the
	/// `<app>--<stage>--analytics` DynamoDB table the deploy also derives.
	#[entities]
	pub store: Option<Entity>,
}

impl Default for AnalyticsConfig {
	fn default() -> Self {
		Self {
			record_requests: true,
			store_ip: false,
			store: None,
		}
	}
}

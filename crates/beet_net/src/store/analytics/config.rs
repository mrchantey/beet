//! The [`AnalyticsConfig`] on-switch component.
use beet_core::prelude::*;

/// Per-router analytics settings, and the on-switch: spawning it resolves the
/// store, so a beet app records nothing until one is present.
///
/// A component on the router entity, authorable from markup as
/// `<AnalyticsConfig/>`. Terminal page views and web beacons persist once it
/// exists; the fields tune the [`AnalyticsKind::Request`](super::AnalyticsKind)
/// stream the router middleware records.
///
/// The store it records to is named by a
/// [`TableStoreRef`](crate::prelude::TableStoreRef) on the same entity, pointing
/// at the entity that *declares* the store, ie
///
/// ```html
/// <DynamoTableBlock bx:ref="analytics" label="analytics"/>
/// <Router {(AnalyticsConfig, TableStoreRef($analytics))}>..</Router>
/// ```
///
/// The reference is required: a config with nowhere to record is the failure
/// this design exists to make unrepresentable, so it is a loud error rather
/// than a silent fallback.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AnalyticsConfig {
	/// Record a request event per routed request. On by default; set `false` to
	/// keep only the client-reported streams.
	pub record_requests: bool,
	/// Store the raw client ip on events. Off by default, so the default posture
	/// derives only a country and collects no personal data.
	pub store_ip: bool,
}

impl Default for AnalyticsConfig {
	fn default() -> Self {
		Self {
			record_requests: true,
			store_ip: false,
		}
	}
}

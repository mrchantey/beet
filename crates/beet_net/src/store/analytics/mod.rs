//! Cross-transport analytics: recording and storing what kinds of clients
//! connect, what pages they visit, and for how long.
//!
//! One [`AnalyticsEvent`] type (built on beet's [`Value`], not `serde_json`)
//! spans every transport, stored in a [`TableStore`]. Its [`AnalyticsEventKind`]
//! discriminates the [`AnalyticsEventData`] payload: a `Request` is the raw
//! server traffic log, a `PageView` a viewed page with a dwell duration, and the
//! client also reports `Click` / `Scroll` / `Error` interactions.
//!
//! Emitters just `trigger` an [`AnalyticsEvent`]; the single persistence observer
//! stores it. The server-side request middleware and the in-world navigator
//! emitters live in beet_router; this module owns the wire types, the store, the
//! geoip country lookup, and the [`analytics_ext`] helpers.

// the types + emission need only serde (via `std`); `Value`, `Uuid`, the event
// enum and the geoip lookup are not json.
mod config;
pub use config::*;
mod event;
pub use event::*;
mod geoip;
pub use geoip::*;
mod retention;
pub use retention::*;
mod rollup;
pub use rollup::*;
mod summary;
pub use summary::*;
pub mod analytics_ext;
// the store persistence rides the json `TableStore` surface, and the cold
// archive rides its ndjson.
#[cfg(feature = "json")]
mod archive;
#[cfg(feature = "json")]
pub use archive::*;
#[cfg(feature = "json")]
mod rollup_job;
#[cfg(feature = "json")]
pub use rollup_job::*;
#[cfg(feature = "json")]
mod store;
#[cfg(feature = "json")]
use beet_core::prelude::*;
#[cfg(feature = "json")]
pub use store::*;

/// Plugin that wires analytics: the storage backend, the persistence observer,
/// and the geoip country database.
///
/// Inert until an [`AnalyticsConfig`] is spawned (the on-switch): its insertion
/// creates the store on that same entity, so a plain beet app with this plugin
/// still does nothing. Once a config is present, terminal page views and web
/// beacons persist automatically; the per-request
/// [`AnalyticsEventKind::Request`] log additionally honors the config (recording
/// on by default, raw ip off by default).
///
/// [`GeoIpDb`] is the separate, optional country-database declaration, registered
/// unconditionally: a binary built without the `geoip` feature still authors it
/// and simply loads an empty [`GeoIp`].
#[cfg(feature = "json")]
pub fn analytics_plugin(app: &mut App) {
	app.register_type::<AnalyticsConfig>()
		.register_type::<AnalyticsRetention>()
		.register_type::<GeoIpDb>()
		// the nightly job and the two store relations it names its aggregate
		// table and its archive by, so `<Route path="rollup" {(
		// AnalyticsRollupJob, RollupStoreRef($rollup))}/>` authors from markup.
		.register_type::<AnalyticsRollupJob>()
		.register_type::<RollupStoreRef>()
		.register_type::<RollupStoreConsumers>()
		.register_type::<ArchiveStoreRef>()
		.register_type::<ArchiveStoreConsumers>()
		.add_observer(GeoIpDb::load_on_add)
		.add_observer(store::spawn_store_on_config)
		.add_observer(store::handle_analytics_event);
}

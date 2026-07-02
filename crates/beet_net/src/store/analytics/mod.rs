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
mod summary;
pub use summary::*;
pub mod analytics_ext;
// the store persistence rides the json `TableStore` surface.
#[cfg(feature = "json")]
mod store;
#[cfg(feature = "json")]
pub use store::*;
#[cfg(feature = "json")]
use beet_core::prelude::*;

/// Plugin that wires analytics: the storage backend, the persistence observer,
/// and the geoip country database.
///
/// Inert until an [`AnalyticsConfig`] is spawned (the on-switch): its insertion
/// creates the store, so a plain beet app with this plugin still does nothing.
/// Once a config is present, terminal page views and web beacons persist
/// automatically; the per-request [`AnalyticsEventKind::Request`] log
/// additionally honors the config (recording on by default, raw ip off by
/// default).
#[cfg(feature = "json")]
pub fn analytics_plugin(app: &mut App) {
	app.register_type::<AnalyticsConfig>()
		.add_observer(store::spawn_store_on_config)
		.add_observer(store::handle_analytics_event);
}

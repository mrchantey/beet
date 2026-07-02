//! Cross-transport analytics: recording and storing what kinds of clients
//! connect, what pages they visit, and for how long.
//!
//! One [`AnalyticsEvent`] type (built on beet's [`Value`], not `serde_json`)
//! spans every transport, stored in a [`TableStore`]. Kinds share it: a
//! [`AnalyticsKind::Request`] is the raw server traffic log; a
//! [`AnalyticsKind::PageView`] is a viewed page with a dwell duration; and the
//! client also reports [`AnalyticsKind::Click`] / [`AnalyticsKind::Scroll`] /
//! [`AnalyticsKind::Error`] interactions.
//!
//! Emitters just `trigger` an [`AnalyticsEvent`]; the single persistence observer
//! stores it. The server-side request middleware and the in-world navigator
//! emitters live in beet_router; this module owns the wire types, the store, the
//! [`geoip`](self::country) country lookup, and the [`analytics_ext`] helpers.
use beet_core::prelude::*;

mod config;
pub use config::*;
mod event;
pub use event::*;
mod geoip;
pub use geoip::*;
mod store;
pub use store::*;
mod summary;
pub use summary::*;
pub mod analytics_ext;

/// Plugin that wires analytics: the storage backend, the persistence observer,
/// and the geoip country database.
///
/// Inert until an [`AnalyticsConfig`] is spawned (the on-switch): its insertion
/// creates the store, so a plain beet app with this plugin still does nothing.
/// Once a config is present, terminal page views and web beacons persist
/// automatically; the per-request [`AnalyticsKind::Request`] log additionally
/// honors the config (recording on by default, raw ip off by default).
pub fn analytics_plugin(app: &mut App) {
	app.register_type::<AnalyticsConfig>()
		.add_observer(store::spawn_store_on_config)
		.add_observer(store::handle_analytics_event);
}

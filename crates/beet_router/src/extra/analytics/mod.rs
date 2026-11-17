//! The router-side analytics integration: the request middleware, the web
//! beacon route, the in-world navigator page-view recorder, and the
//! [`VisitCount`] a page reads back.
//!
//! The wire types, store, and geoip live in beet_net's `store::analytics`; this
//! module is the emitters that feed them. All three record into the same
//! [`AnalyticsEvent`](beet_net::prelude::AnalyticsEvent).
// the request middleware + navigator recorder build the typed event (serde, std);
// only the beacon route parses a json request body, so it rides `json`.
mod analytics_middleware;
pub use analytics_middleware::*;
mod navigator_analytics;
pub(crate) use navigator_analytics::*;
#[cfg(feature = "json")]
mod analytics_handler;
pub mod router_analytics_ext;
#[cfg(feature = "json")]
pub(crate) use analytics_handler::*;
// the count reads the json segments and aggregates back through the store.
#[cfg(feature = "json")]
mod visit_count;
#[cfg(feature = "json")]
pub use visit_count::*;

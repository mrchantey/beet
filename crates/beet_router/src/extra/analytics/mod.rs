//! The router-side analytics integration: the request middleware, the web
//! beacon route, and the in-world navigator page-view recorder.
//!
//! The wire types, store, and geoip live in beet_net's `store::analytics`; this
//! module is the emitters that feed them. All three record into the same
//! [`AnalyticsEvent`](beet_net::prelude::AnalyticsEvent).
mod analytics_handler;
pub use analytics_handler::*;
mod analytics_middleware;
pub use analytics_middleware::*;
mod navigator_analytics;
pub use navigator_analytics::*;
pub mod router_analytics_ext;

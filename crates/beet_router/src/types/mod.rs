//! Core types for the beet_router system.
//!
//! This module provides the foundational types for routing:
//!
//! - [`PathPattern`]: URL path matching patterns with wildcards and parameters
//! - [`ParamsPattern`]: Query and path parameter definitions
//! - [`EndpointTree`]: Hierarchical route organization
//! - [`RouteQuery`]: Request routing and matching
//! - [`BodyType`]: Request/response body type metadata
//! - [`RouterPlugin`]: Bevy plugin for router integration

mod body_type;
#[cfg(feature = "server")]
mod default_router;
mod endpoint_tree;
mod route_query;
pub use body_type::*;
pub use collect_html::*;
#[cfg(feature = "server")]
pub use default_router::*;
pub use endpoint_tree::*;
pub use route_query::*;
pub use server_action_request::*;
mod collect_html;
mod router_plugin;
mod server_action_request;
pub use router_plugin::*;

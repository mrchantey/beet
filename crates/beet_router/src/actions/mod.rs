//! Actions and components for HTTP request routing.
//!
//! This module provides the core building blocks for defining routes:
//!
//! - [`Endpoint`]: Terminal route handler with method/path matching
//! - [`EndpointBuilder`]: Fluent API for constructing endpoints
//! - [`RouterExchange`]: Exchange pattern for routing requests through a tree
//! - [`common_middleware`]: Common middleware like CORS, logging, etc.
//! - [`common_predicates`]: Route matching predicates
//! - [`html_bundle`]: HTML response construction utilities

mod bucket_endpoint;
pub mod common_middleware;
pub mod common_predicates;
mod help_handler;
pub mod html_bundle;
mod router_exchange;
pub use bucket_endpoint::*;
pub use help_handler::*;
pub use html_bundle::*;
pub use router_exchange::*;
mod endpoint_builder;
pub use endpoint_builder::*;
mod endpoint;
pub use endpoint::*;
mod server_action;
pub use server_action::*;
mod endpoint_action;
pub use common_middleware::CorsConfig;
pub use endpoint_action::*;

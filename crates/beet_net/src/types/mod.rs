//! Transport-agnostic request/response types and exchange patterns.
//!
//! This module provides the core HTTP-like types ([`Request`], [`Response`],
//! [`RequestParts`], [`ResponseParts`], etc.) as well as higher-level exchange
//! patterns built on the action system.
//!
//! ## Core Types
//!
//! - [`Request`] / [`Response`]: Generalized request/response types
//! - [`RequestParts`] / [`ResponseParts`]: Transport-agnostic metadata
//! - [`StatusCode`] / [`HttpMethod`]: HTTP status codes and methods
//! - [`HeaderMap`] / [`MediaType`]: Header management and content types
//! - [`Url`] / [`Scheme`]: URL representation and transport scheme
//! - [`Body`]: Request/response body (bytes or stream)
//! - [`PathPattern`] / [`ParamsPattern`]: URL pattern matching
//!
//! ## Exchange Patterns
//!
//! - [`ExchangeExt`] / [`AsyncExchangeExt`]: Convenience traits for calling
//!   `Action<Request, Response>` on entities
//! - [`ExchangeEnd`]: Event triggered when an exchange completes
//! - [`exchange_handler`]: Creates a sync `Action<Request, Response>` from a closure
//! - [`exchange_handler_async`]: Creates an async `Action<Request, Response>` from a closure
//! - [`mirror_exchange`]: Echoes requests back as responses

// core types (moved from beet_core)
mod body;
pub mod header;
/// Alias for [`header`] for ergonomic typed header access.
pub use header as headers;
mod header_map;
mod media_bytes_ext;
mod parts;
mod request;
mod response;
mod url;
pub use body::*;
pub use header_map::*;
pub use media_bytes_ext::*;
pub use response::*;
pub use url::*;
mod param_pattern;
pub use param_pattern::*;
mod path_pattern;
pub use parts::*;
pub use path_pattern::*;
pub use request::*;
mod http_error;
pub use http_error::*;
mod rel_path_ext;
pub use rel_path_ext::*;
mod param_query;
pub use param_query::*;
mod http_method;
mod status_code;
pub use http_method::*;
pub use status_code::*;
#[cfg(feature = "http")]
pub mod http_ext;

// higher-level exchange patterns
mod exchange;
mod exchange_stats;
mod extractors;
pub use exchange::*;
pub use exchange_stats::*;
pub use extractors::*;

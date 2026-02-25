//! Request/response exchange patterns built on the tool system.
//!
//! This module provides infrastructure for handling request/response
//! exchanges within the Bevy ECS using [`Tool<Request, Response>`]
//! components from `beet_tool`.
//!
//! ## Core Concepts
//!
//! - [`ExchangeExt`] / [`AsyncExchangeExt`]: Convenience traits for calling
//!   `Tool<Request, Response>` on entities
//! - [`ExchangeEnd`]: Event triggered when an exchange completes
//!
//! ## Exchange Patterns
//!
//! - [`handler_exchange`]: Creates a sync `Tool<Request, Response>` from a closure
//! - [`handler_exchange_async`]: Creates an async `Tool<Request, Response>` from a closure
//! - [`mirror_exchange`]: Echoes requests back as responses
mod exchange;
mod exchange_stats;
mod extractors;
mod handler_exchange;
pub use exchange::*;
pub use exchange_stats::*;
pub use extractors::*;
pub use handler_exchange::*;

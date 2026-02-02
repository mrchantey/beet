//! Generalized request/response exchange patterns for Bevy entities.
//!
//! This module provides infrastructure for handling transport agnostic request/response
//! exchanges within the Bevy ECS. It enables entities to act as "servers" that
//! receive requests and produce responses through the observer pattern.
//!
//! ## Core Concepts
//!
//! - [`ExchangeStart`]: Event triggered when a request arrives at an entity
//! - [`ExchangeContext`]: Contains the response sender and timing information
//! - [`ExchangeEnd`]: Event triggered when an exchange completes
//!
//! ## Exchange Patterns
//!
//! - [`SpawnExchange`]: Spawns a child entity for each exchange
//! - [`HandlerExchange`]: Routes requests to handler functions
//! - [`FlowExchange`]: Integrates with beet_flow for behavior tree responses
mod exchange;
mod exchange_stats;
mod extractors;
#[cfg(feature = "flow")]
mod flow_exchange;
#[cfg(feature = "flow")]
mod flow_exchange_stream;
mod handler_exchange;
mod spawn_exchange;
pub use exchange::*;
pub use exchange_stats::*;
pub use extractors::*;
#[cfg(feature = "flow")]
pub use flow_exchange::*;
#[cfg(feature = "flow")]
pub use flow_exchange_stream::*;
pub use handler_exchange::*;
pub use spawn_exchange::*;

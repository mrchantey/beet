//! Browser and WebAssembly utilities for DOM manipulation and JS interop.
//!
//! This module provides cross-platform abstractions for working with the browser
//! environment, including DOM manipulation, event handling, file uploads/downloads,
//! and History API helpers.
//!
//! # Modules
//!
//! - [`document_ext`] - Helpers for creating and querying DOM elements
//! - [`element_ext`] - Element-scoped query and manipulation utilities
//! - [`file_ext`] - File upload and download utilities
//! - [`history_ext`] - Browser History API wrappers
//! - [`search_params_ext`] - URL query string manipulation
//! - [`lifecycle_ext`] - Async timeout and lifecycle utilities
//! - [`js_runtime`] - Deno/browser runtime bindings for file I/O and environment
//! - [`array_ext`] - JS array conversion helpers
//! - [`iframe_ext`] - IFrame manipulation and loading utilities

mod animation_frame;
pub use self::animation_frame::*;
/// Helpers for converting JS arrays to Rust collections.
pub mod array_ext;
/// File upload and download utilities for the browser.
pub mod file_ext;
/// Browser History API helpers for SPA navigation.
pub mod history_ext;
mod html_event_listener;
pub use self::html_event_listener::*;
mod interval;
pub use self::interval::*;
/// Runtime bindings for Deno/browser environments.
pub mod js_runtime;
/// Async timeout and lifecycle utilities.
pub mod lifecycle_ext;
mod poll;
pub use self::poll::*;
mod resize_listener;
pub use self::resize_listener::*;
/// URL query string reading and writing utilities.
pub mod search_params_ext;
mod time_utils;
pub use self::time_utils::*;
mod result;
pub use result::*;
mod closure;
pub use closure::*;
/// Document-level DOM helpers for element creation and queries.
pub mod document_ext;
/// Element-scoped DOM query and manipulation utilities.
pub mod element_ext;
/// IFrame manipulation and async load helpers.
pub mod iframe_ext;

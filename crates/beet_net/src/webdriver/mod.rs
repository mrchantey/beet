//! WebDriver integration for automated browser testing.
//!
//! This module provides a WebDriver BiDi protocol implementation for
//! automated browser testing, supporting both Chrome and Firefox.
//!
//! # Features
//!
//! - Session management for browser automation
//! - Page navigation and interaction
//! - Element querying and manipulation
//! - PDF export functionality

mod client;
mod element;
mod export_pdf;
mod page;
mod session;

pub use client::*;
pub use element::*;
pub use export_pdf::*;
pub use page::*;
pub use session::*;

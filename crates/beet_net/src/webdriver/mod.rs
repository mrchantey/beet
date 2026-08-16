//! WebDriver integration for automated browser testing.
//!
//! This module provides a WebDriver BiDi protocol implementation for
//! automated browser testing, supporting both Chrome and Firefox.
//!
//! # Features
//!
//! - Session management with typed event subscriptions
//! - Page navigation, script evaluation and auto-waiting element location
//! - Trusted pointer/key input and console/network collectors
//! - Screenshot and PDF export

mod bidi_value;
mod browser;
mod client;
mod collector;
mod element;
mod export_pdf;
mod input;
mod locate;
#[cfg(any(test, feature = "testing"))]
mod matchers;
mod page;
mod screenshot;
mod session;
#[cfg(test)]
mod test_fixtures;

pub use browser::*;
pub use client::*;
pub use collector::*;
pub use element::*;
pub use export_pdf::*;
pub use page::*;
pub use screenshot::*;
pub use session::*;

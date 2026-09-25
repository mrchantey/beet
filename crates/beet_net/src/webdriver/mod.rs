//! WebDriver integration for automated browser testing.
//!
//! This module provides a WebDriver BiDi protocol implementation for
//! automated browser testing, supporting both Chrome and Firefox.
//!
//! # Features
//!
//! - Session management with typed event subscriptions
//! - Page navigation, script evaluation and auto-waiting element location
//! - Trusted pointer/key input (a secret typed without a trace) and
//!   console/network collectors, with a network-quiet drain
//! - The browser's cookies, for a plain http client to carry its session
//! - A persistent profile, so a login outlives the session
//! - Screenshot and PDF export

mod bidi_value;
mod browser;
mod client;
mod collector;
mod cookies;
mod element;
mod export_pdf;
// the serve-a-bundle harness runs a real listener, so it needs the server half
// beside the webdriver half
#[cfg(all(any(test, feature = "testing"), feature = "server"))]
mod harness;
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
pub use cookies::*;
pub use element::*;
pub use export_pdf::*;
#[cfg(all(any(test, feature = "testing"), feature = "server"))]
pub use harness::*;
pub use page::*;
pub use screenshot::*;
pub use session::*;

//! Web-target widgets: the ones that emit a `<style>` or `<script>` element for
//! a browser and are inert elsewhere.
//!
//! [`Preflight`]/[`Reset`] normalize browser defaults, [`Stylesheet`] bakes the
//! active rule set into the document, [`ColorSchemeScript`] seeds the color
//! scheme before first paint, and [`Analytics`] injects the analytics script.
#[cfg(feature = "net")]
mod analytics;
mod browser_reset;
mod color_scheme;
#[cfg(feature = "style")]
mod stylesheet;

#[cfg(feature = "net")]
pub use analytics::*;
pub use browser_reset::*;
pub use color_scheme::*;
#[cfg(feature = "style")]
pub use stylesheet::*;

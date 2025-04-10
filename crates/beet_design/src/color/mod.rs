//! Color is a deceptively complex topic.
//!
//! ## Definitions:
//!
//! - Theme: The set of all colors used in the application.
//! - Scheme: A subset of the theme with a specific purpose. ie `light`, `dark`.
//!
//!
mod color_scheme;
mod theme_to_css;
pub use color_scheme::*;
pub use theme_to_css::*;

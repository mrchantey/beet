//! Color is a deceptively complex topic.
//!
//! ## Definitions:
//!
//! - Theme: The set of all colors used in the application.
//! - Scheme: A subset of the theme with a specific purpose. ie `light`, `dark`.
//!
//!
mod color_theme;
mod theme_to_css;
pub use color_theme::*;
pub use theme_to_css::*;

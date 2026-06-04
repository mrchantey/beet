//! Terraform config types and utilities.
mod config;
mod ident;
mod misc;
mod project;
pub mod tofu;
pub use config::*;
pub use ident::*;
pub use misc::*;
pub use project::*;

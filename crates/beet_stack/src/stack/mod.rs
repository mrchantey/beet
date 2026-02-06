//! This module defines the arrangement of applications into something
//! like a filesystem, comprised of a combination of [`Stack`] and [`Card`]
//!
//! ## Interface Agnostic
//!
//! The `Stack/Card` metaphore is directly inspired by `hypercard`, but is
//! different in that it may not nessecarily be represented as a user interface.
//! The tools and content in a [`Card`] may be exposed as router endpoints, cli subcommands,
//! or mcp resources and tools.
//!
//!
mod stack;
pub use stack::*;

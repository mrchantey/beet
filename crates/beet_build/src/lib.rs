//! Build-time code generation and live reloading for beet applications.
//!
//! This crate provides the infrastructure for parsing source files, extracting RSX snippets,
//! and generating route code. It supports both Rust (`.rs`) and Markdown (`.md`, `.mdx`) files.
//!
//! # Architecture
//!
//! The build process involves several stages:
//!
//! ## Source Files
//! - **Startup**: For every file specified in the [`WorkspaceConfig`], load a [`SourceFile`]
//! - **Create**: When a file is created, spawn a corresponding [`SourceFile`] entity
//! - **Remove**: When a file is removed, despawn matching [`SourceFile`] entities
//! - **Modify**: When a file changes, mark it as changed and despawn all children
//!
//! ## RSX Snippets
//!
//! Load [`RsxSnippetOf`](beet_dom::prelude::SnippetRoot) for each [`SourceFile`].
//!
//! ## Route Codegen
//!
//! Reparent any [`SourceFile`] to its matching [`RouteFileCollection`] if any.
//!
//! ```text
//! // source files that do not appear in a route collection
//! SourceFileRoot
//! ├── (SourceFile(foo.rs), FileExprHash)
//! │   ├── (StaticRoot, RsxSnippetOf, RsxTokens)
//! └── (SourceFile(foo.md), FileExprHash)
//!     └── (StaticRoot, RsxSnippetOf, CombinatorTokens)
//!
//! // route collections are a separate tree
//! RouteFileCollection
//! ├── (SourceFileRef(foo.rs), CodegenFile RouteFile)
//! │   │   // an entity for each route in a file (get, post, etc)
//! │   ├── (RouteFileMethod)
//! │   └── (RouteFileMethod)
//! └── (SourceFileRef(foo.md), CodegenFile RouteFile)
//!     │   // markdown files have a single 'get' route
//!     ├── (RouteFileMethod)
//!     │   // generate the rust code for markdown files
//!     └── (SourceFileRef(foo.md), CodegenFile, CombinatorRouteCodegen)
//! ```
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(if_let_guard, exit_status_error)]
#![deny(missing_docs)]
#[allow(unused)]
use crate::prelude::*;


mod actions;
mod route_codegen;
mod snippets;
mod utils;

/// Re-exports of commonly used build types and utilities.
pub mod prelude {
	pub use crate::actions::*;
	pub use crate::route_codegen::*;
	pub use crate::snippets::*;
	pub use crate::utils::*;
}

/// Re-exports of external crates used by beet_build.
pub mod exports {
	pub use proc_macro2;
	pub use proc_macro2_diagnostics;
	pub use syn;
}

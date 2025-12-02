//! This module is responsible for codegen and live reloading.
//!
//! There are several steps involved:
//! ## Source Files
//! - Startup - For every file specified in the [`WorkspaceConfig`], load a [`SourceFile`]
//! - WatchEvent::Create - create a new corresponding [`SourceFile`]
//! - WatchEvent::Remove - remove every matching [`SourceFile`]
//! - WatchEvent::Modify - find the root source file and mark it as [`Changed`], despawning all children
//!
//! ## Rsx Snippets
//!
//! Load [`RsxSnippets`] for each [`SourceFile`],
//!
//! ## RouteCodegen
//!
//! - Reparent any [`SourceFile`] to its matching [`RouteFileCollection`] if any
//!
//! Higher level parsing than beet_parse, and downstream from beet_rsx and beet_build.
//!
//! ```ignore
//! // source files that do not appear in a route collection
//! SourceFileRoot
//! ├── (SourceFile(foo.rs), FileExprHash)
//! │   ├── (StaticRoot, RsxSnippetOf, RsxTokens)
//! └── (SourceFile(foo.md), FileExprHash)
//!     └── (StaticRoot, RsxSnippetOf, CombinatorTokens)
//!
//! // route collections are a seperate tree
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
//!
//!
//!
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(if_let_guard, exit_status_error)]
#[allow(unused)]
use crate::prelude::*;


mod cli;
mod route_codegen;
mod snippets;
// mod templates;
mod utils;

pub mod prelude {
	pub use crate::cli::*;
	pub use crate::route_codegen::*;
	pub use crate::snippets::*;
	// pub use crate::templates::*;
	pub use crate::utils::*;
}
pub mod exports {
	pub use proc_macro2;
	pub use proc_macro2_diagnostics;
	pub use syn;
}

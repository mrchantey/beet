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
#![feature(let_chains, if_let_guard, result_flattening, exit_status_error)]

mod route_codegen;
mod snippets;
mod utils;

pub mod prelude {
	pub use crate::route_codegen::*;
	pub use crate::snippets::*;
	pub use crate::utils::*;
}
pub mod exports {
	pub use proc_macro2;
	pub use proc_macro2_diagnostics;
	pub use syn;
}
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as build;
		pub use beet_parse as parse;
		pub use beet_rsx as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_core::prelude::*;
			pub use beet_parse::prelude::*;
			pub use beet_rsx::prelude::*;
			pub use beet_utils::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
			pub use beet_rsx::exports::*;
		}
	}
}

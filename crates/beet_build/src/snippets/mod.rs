//! RSX snippet parsing and source file management.
//!
//! This module handles:
//! - Loading and parsing source files (Rust, Markdown)
//! - Extracting RSX snippets from source code
//! - Computing file expression hashes for change detection
//! - Exporting snippets for client-side hydration

mod export_snippets;
mod file_expr_hash;
mod hash_non_snippet_rust;
mod import_file_inner_text;
mod import_rsx_snippets_md;
mod import_rsx_snippets_rs;
mod parse_markdown;
mod source_file;
mod syn_serde;

pub use file_expr_hash::*;
pub use source_file::*;

pub(crate) use export_snippets::*;
pub(crate) use import_file_inner_text::*;
pub(crate) use import_rsx_snippets_md::*;
pub(crate) use import_rsx_snippets_rs::*;
pub(crate) use parse_markdown::*;

/// Error types for snippet operations.
pub mod error;

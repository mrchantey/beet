//! The single `rsx!` markup macro: a vendored, panic-free, source-text-free
//! parser and the lowering to a [`Bundle`] tree.
//!
//! - [`parse`]: `TokenStream` -> [`ast`], recoverable and IDE-friendly.
//! - [`lower`]: [`ast`] -> a `Bundle` token stream.
//! - [`rsx_macro`]: the `rsx!` entry tying the two together.
//!
//! This mirrors the runtime BSX parser in `beet_core/src/bsx` (`ast` / `parse`)
//! but stays a separate codebase: BSX consumes `&str`, this consumes
//! `proc_macro2::TokenStream`. Shared vocabulary, not shared code.
mod ast;
mod lower;
mod parse;
mod rsx_macro;

pub use rsx_macro::impl_rsx;

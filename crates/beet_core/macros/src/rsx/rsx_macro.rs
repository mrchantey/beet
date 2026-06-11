//! The `rsx!` proc-macro entry: parse markup, lower it, wrap in a [`Snippet`].
//!
//! Lowering targets the `Element`/`Attribute`/`children!`/`Value` base and wraps
//! the result at the root in `Snippet::from_bundle(..)`, yielding the
//! `impl Template<Output = ()>` (and `impl Bundle`) that `spawn_template` accepts.
//! The grammar and lowering rules live in [`super::parse`] and [`super::lower`].
use super::lower::lower_nodes;
use super::parse::parse_rsx;
use alloc::vec::Vec;
use beet_core_shared::pkg_ext;
use proc_macro2::TokenStream;
use quote::quote;

/// The `rsx!` proc-macro entry: lowers markup to an `impl Template<Output = ()>`
/// by wrapping the lowered bundle in `Snippet::from_bundle(..)`.
pub fn impl_rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let (nodes, errors) = parse_rsx(input.into());
	let error_tokens: Vec<TokenStream> = errors
		.into_iter()
		.map(|err| err.emit_as_expr_tokens())
		.collect();
	let bundle = lower_nodes(&nodes);
	quote! {{
		use #beet_core::prelude::*;
		#(#error_tokens)*
		Snippet::from_bundle(#bundle)
	}}
	.into()
}

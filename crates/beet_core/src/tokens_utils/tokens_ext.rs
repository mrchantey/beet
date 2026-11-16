//! Type-to-token utilities for proc macros and codegen.

use crate::prelude::*;

/// The [`TypeName::short`] of `T` as a [`syn::Path`], ie `Option<Vec<usize>>`
/// for `core::option::Option<alloc::vec::Vec<usize>>`; the caller brings the
/// type into scope.
pub fn short_type_path<T>() -> syn::Path {
	let short_name = TypeName::of::<T>().short();
	syn::parse_str::<syn::Path>(&short_name).expect(&format!(
		"Failed to parse type name {short_name} into syn::Path"
	))
}

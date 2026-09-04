//! Type-to-token utilities for proc macros and codegen.

use crate::prelude::*;

/// Returns the last part of an [`std::any::type_name`] as a [`syn::Path`],
/// the user is expected to bring the type into scope.
/// Where the typename is `"std::option::Option<std::vec::Vec<usize>>"`,
/// the output is `Option<Vec<usize>>`
pub fn short_type_path<T>() -> syn::Path {
	let short_name = type_ext::short_name::<T>();
	syn::parse_str::<syn::Path>(&short_name).expect(&format!(
		"Failed to parse type name {short_name} into syn::Path"
	))
}

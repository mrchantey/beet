//! Route file method representation for individual HTTP handlers.
//!
//! This module defines the [`RouteFileMethod`] component that represents
//! a single HTTP route handler function extracted from a source file.

#[allow(unused_imports)]
use crate::prelude::*;
use beet_core::prelude::*;
use syn::Ident;
use syn::ItemFn;

/// Represents an HTTP route handler function extracted from a source file.
///
/// Each route file may contain multiple route methods corresponding to
/// different HTTP methods (GET, POST, etc.).
///
/// # Examples
///
/// A Rust source file might define:
/// ```ignore
/// pub fn get() -> impl Bundle {
///     // ...
/// }
///
/// pub fn post() -> impl IntoResponse {
///     // ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct RouteFileMethod {
	/// The route path generated from this file's local path.
	pub path: RoutePath,
	/// The HTTP method matching either the function's signature, or
	/// `GET` in the case of single-file routes like Markdown.
	pub method: HttpMethod,
	/// The parsed function signature of the route handler.
	pub item: Unspan<ItemFn>,
}

impl AsRef<RouteFileMethod> for RouteFileMethod {
	fn as_ref(&self) -> &RouteFileMethod { self }
}


impl RouteFileMethod {
	/// Creates a new route file method with the given path and method,
	/// using a default function signature matching the method name.
	pub fn new(path: impl Into<RoutePath>, method: HttpMethod) -> Self {
		let path = path.into();
		let method_name = method.to_string_lowercase();
		let method_ident = quote::format_ident!("{method_name}");
		Self {
			path,
			method,
			item: Unspan::new(&syn::parse_quote!(
				fn #method_ident() {}
			)),
		}
	}

	/// Creates a new route file method with a custom function signature.
	pub fn new_with(
		path: impl Into<RoutePath>,
		method: HttpMethod,
		item: &ItemFn,
	) -> Self {
		Self {
			path: path.into(),
			method,
			item: Unspan::new(item),
		}
	}

	/// Creates a route file method from a file path.
	///
	/// The route path is derived from the file path using [`RoutePath::from_file_path`].
	pub fn from_path(
		local_path: impl AsRef<std::path::Path>,
		method: HttpMethod,
	) -> Self {
		let route = RoutePath::from_file_path(local_path).unwrap();
		Self::new(route, method)
	}

	/// Returns `true` if the return type of this method is a `Result`.
	pub fn returns_result(&self) -> bool {
		if let syn::ReturnType::Type(_, ty) = &self.item.sig.output {
			if let syn::Type::Path(syn::TypePath { path, .. }) = &**ty {
				if let Some(seg) = path.segments.last() {
					return seg.ident == "Result";
				}
			}
		}
		false
	}
}

/// Specifies the location of metadata for a route file method.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum RouteFileMethodMeta {
	/// A config method exists for this specific route file method,
	/// ie `my_route::config_post()`.
	Method,
	/// A config method exists for this route file, ie `my_route::config()`.
	File,
	/// No config method exists for this route file; fall back to
	/// the collection level or default.
	#[default]
	Collection,
}

impl RouteFileMethodMeta {
	/// Returns the path to the meta function for this route file method.
	///
	/// This path can be called to get the metadata for the route.
	pub fn ident(&self, mod_ident: &Ident, method_name: &str) -> syn::Path {
		match self {
			RouteFileMethodMeta::Method => {
				let meta_ident = quote::format_ident!("meta_{}", method_name);
				syn::parse_quote!(#mod_ident::#meta_ident)
			}
			RouteFileMethodMeta::File => syn::parse_quote!(#mod_ident::meta),
			RouteFileMethodMeta::Collection => syn::parse_quote!(Self::meta),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn returns_result() {
		RouteFileMethod::new_with(
			"",
			HttpMethod::Get,
			&syn::parse_quote!(
				fn get() {}
			),
		)
		.returns_result()
		.xpect_false();
		RouteFileMethod::new_with(
			"",
			HttpMethod::Get,
			&syn::parse_quote!(
				fn get() -> Result<(), ()> {}
			),
		)
		.returns_result()
		.xpect_true();
	}
}

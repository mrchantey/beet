#[allow(unused_imports)]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use syn::Ident;
use syn::ItemFn;

/// Tokens for a function that may be used as a route:
///
/// ```ignore
/// pub fn get()->impl Bundle{
/// ..
/// }
///
/// pub fn post()->impl IntoResponse{
/// ..
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct RouteFileMethod {
	/// A reasonable route path generated from this file's local path
	pub path: RoutePath,
	/// The HTTP method matching either the functions signature, or
	/// `get` in the case of single file routes like markdown.
	pub method: HttpMethod,
	/// The signature of a route file method
	pub item: Unspan<ItemFn>,
}
impl AsRef<RouteFileMethod> for RouteFileMethod {
	fn as_ref(&self) -> &RouteFileMethod { self }
}


impl RouteFileMethod {
	/// create a new route file method with the given path and method
	/// and a default function signature matching the method name.
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

	pub fn from_path(
		local_path: impl AsRef<std::path::Path>,
		method: HttpMethod,
	) -> Self {
		let route = RoutePath::from_file_path(local_path).unwrap();
		Self::new(route, method)
	}

	/// Whether the return type of this method is a `Result`.
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

/// Specify the location of the meta for this route file method.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum RouteFileMethodMeta {
	/// A config method exists for this route file method,
	/// ie `my_route::config_post()`.
	Method,
	/// A config method exists for this route file, ie `my_route::config()`.
	File,
	/// No config method exists for this route file, fall back to
	/// the collection level or default.
	#[default]
	Collection,
}

impl RouteFileMethodMeta {
	/// Returns the path to the meta function for this route file method,
	/// which can be called to get the metadata for the route.
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
	use beet_net::prelude::*;
	use sweet::prelude::*;

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

#[allow(unused_imports)]
use crate::prelude::*;
use beet_core::as_beet::*;
use bevy::prelude::*;
use syn::Ident;
use syn::ItemFn;

/// Tokens for a function that may be used as a route.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct RouteFileMethod {
	/// A reasonable route path generated from this file's local path,
	/// and a method matching either the functions signature, or
	/// `get` in the case of single file routes like markdown.
	pub route_info: RouteInfo,
	/// The signature of a route file method
	pub item: Unspan<ItemFn>,
}
impl AsRef<RouteFileMethod> for RouteFileMethod {
	fn as_ref(&self) -> &RouteFileMethod { self }
}


impl RouteFileMethod {
	/// create a new route file method with the given route info
	/// and a default function signature matching the method name.
	pub fn new(route_info: impl Into<RouteInfo>) -> Self {
		let route_info = route_info.into();
		let method = route_info.method.to_string_lowercase();
		let method_ident = quote::format_ident!("{method}");
		Self {
			route_info,
			item: Unspan::new(&syn::parse_quote!(
				fn #method_ident() {}
			)),
		}
	}
	pub fn new_with(route_info: impl Into<RouteInfo>, item: &ItemFn) -> Self {
		let route_info = route_info.into();
		Self {
			route_info,
			item: Unspan::new(item),
		}
	}

	pub fn from_path(
		local_path: impl AsRef<std::path::Path>,
		method: HttpMethod,
	) -> Self {
		let route = RoutePath::from_file_path(local_path).unwrap();
		Self::new(RouteInfo::new(route, method))
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

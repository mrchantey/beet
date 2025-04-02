//! Convert a vec of [`FuncFile`] into a vec of [`RouteFuncTokens`].
//! The number of functions is usally different, ie file based routes may
//! have a `get` and `post` function, whereas mockups may merge all
//! functions into one route.


use std::path::PathBuf;

use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;

// const HTTP_METHODS: [&str; 9] = [
// 	"get", "post", "put", "delete", "head", "options", "connect", "trace",
// 	"patch",
// ];

pub struct FuncFilesToRouteFuncs;

impl FuncFilesToRouteFuncs {
	/// Map the [`FuncFile`] to a vec of [`RouteFuncTokens`].
	pub fn map_func_files<T, F>(
		func_files: T,
		func: F,
	) -> Result<(T, Vec<RouteFuncTokens>)>
	where
		T: AsRef<Vec<FuncFile>>,
		F: Fn(&FuncFile) -> Result<Vec<RouteFuncTokens>>,
	{
		let route_funcs = func_files
			.as_ref()
			.iter()
			.map(|func_file| func(func_file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		Ok((func_files, route_funcs))
	}
}

/// Simply map the [`FuncFile`] to a vec of [`RouteFuncTokens`] based on
/// the http methods of the functions in the file.
#[derive(Debug, Default, Clone)]
pub struct HttpFuncFilesToRouteFuncs;


impl<T: AsRef<Vec<FuncFile>>> RsxPipeline<T, Result<(T, Vec<RouteFuncTokens>)>>
	for HttpFuncFilesToRouteFuncs
{
	fn apply(self, func_files: T) -> Result<(T, Vec<RouteFuncTokens>)> {
		FuncFilesToRouteFuncs::map_func_files(func_files, |file| {
			let route_path = file.default_route_path()?;
			let ident = &file.ident;
			let route_path_str = route_path.to_string_lossy();
			file.funcs
				.iter()
				.map(|sig| {
					let sig_ident = &sig.ident;
					let sig_ident_str = sig_ident.to_string();
					let block = syn::parse_quote! {{
					RouteFunc::new(
						#sig_ident_str,
						#route_path_str,
						#ident::#sig_ident
					)}};
					RouteFuncTokens::build(
						&file.local_path,
						route_path.clone(),
						block,
					)
				})
				.collect()
		})
	}
}
#[derive(Debug, Clone)]
pub struct MockupFuncFilesToRouteFuncs {
	/// A base path to prepend to the route path
	base_route: RoutePath,
}
impl Default for MockupFuncFilesToRouteFuncs {
	fn default() -> Self {
		Self {
			base_route: RoutePath::new("/mockups"),
		}
	}
}

impl MockupFuncFilesToRouteFuncs {
	/// Create a new [`MockupFuncFilesToRouteFuncs`] with the given base route.
	pub fn new(base_route: impl Into<PathBuf>) -> Self {
		Self {
			base_route: RoutePath::new(base_route),
		}
	}
}

impl<T: AsRef<Vec<FuncFile>>> RsxPipeline<T, Result<(T, Vec<RouteFuncTokens>)>>
	for MockupFuncFilesToRouteFuncs
{
	fn apply(self, func_files: T) -> Result<(T, Vec<RouteFuncTokens>)> {
		FuncFilesToRouteFuncs::map_func_files(func_files, |file| {
			let route_path = self
				.base_route
				.join(&file.default_route_path()?)
				.to_string_lossy()
				.to_string()
				.replace(".mockup", "");
			let route_path = RoutePath::new(route_path);
			println!("route_path: {:?}", route_path);

			let ident = &file.ident;
			let route_path_str = route_path.to_string_lossy();

			let items = file.funcs.iter().map(|sig| {
				let sig_ident = &sig.ident;
				quote::quote! {#ident::#sig_ident()}
			});
			Ok(vec![RouteFuncTokens::build(
				&file.local_path,
				route_path.clone(),
				syn::parse_quote! {{
					fn func()->RsxNode {
						vec![
							#(#items),*
						].into_node()
					}
					RouteFunc::new(
						"get",
						#route_path_str,
						func
					)
				}},
			)?])
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	// use quote::ToTokens;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let _route_funcs = FileGroup::test_site_routes()
			.bpipe(FileGroupToFuncFiles::default())
			.unwrap()
			.bpipe(HttpFuncFilesToRouteFuncs::default())
			.unwrap();
	}
}

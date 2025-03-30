use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;

// const HTTP_METHODS: [&str; 9] = [
// 	"get", "post", "put", "delete", "head", "options", "connect", "trace",
// 	"patch",
// ];

pub trait MapFuncFileToRoutes {
	fn file_to_routes(&self, file: &FuncFile) -> Result<Vec<RouteFuncTokens>>;
}


/// empty impl so we can create static methods
impl MapFuncFileToRoutes for () {
	fn file_to_routes(&self, _file: &FuncFile) -> Result<Vec<RouteFuncTokens>> {
		Ok(vec![])
	}
}

impl<F: Fn(&FuncFile) -> Result<Vec<RouteFuncTokens>>> MapFuncFileToRoutes
	for F
{
	/// Map the [`FuncFile`] to a vec of [`RouteFuncTokens`].
	/// The ident should be used in conjunction with the [`FuncFile::sigs`] to call
	/// the function, ie `#file_ident::#sig_ident`
	fn file_to_routes(&self, file: &FuncFile) -> Result<Vec<RouteFuncTokens>> {
		self(file)
	}
}

/// Convert a vec of [`FuncFile`] into a vec of [`RouteFuncTokens`].
/// The number of functions is usally different, ie file based routes may
/// have a `get` and `post` function, whereas mockups may merge all
/// functions into one route.
pub struct FuncFilesToRouteFuncs<F: MapFuncFileToRoutes> {
	pub func: F,
}
impl FuncFilesToRouteFuncs<()> {
	/// Simply map the [`FuncFile`] to a vec of [`RouteFuncTokens`] based on
	/// the http methods of the functions in the file.
	pub fn http_routes() -> FuncFilesToRouteFuncs<
		for<'a> fn(&'a FuncFile) -> Result<Vec<RouteFuncTokens>>,
	> {
		fn map(file: &FuncFile) -> Result<Vec<RouteFuncTokens>> {
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
		}

		FuncFilesToRouteFuncs::new(map)
	}
	pub fn mockups() -> FuncFilesToRouteFuncs<
		for<'a> fn(&'a FuncFile) -> Result<Vec<RouteFuncTokens>>,
	> {
		pub fn map(file: &FuncFile) -> Result<Vec<RouteFuncTokens>> {
			let route_path = RoutePath::new("/mockups")
				.join(&file.default_route_path()?)
				.to_string_lossy()
				.to_string()
				.replace(".mockup", "");
			let route_path = RoutePath::new(route_path);

			let ident = &file.ident;
			let route_path_str = route_path.to_string_lossy();

			let items = file.funcs.iter().map(|sig| {
				let sig_ident = &sig.ident;
				quote::quote! {#ident::#sig_ident().node}
			});
			Ok(vec![RouteFuncTokens::build(
				&file.local_path,
				route_path.clone(),
				syn::parse_quote! {{
					fn func()->RsxRoot{
						let node = RsxNode::fragment(vec![
							#(#items),*
						]);
						// TODO we should be able to just return the node,
						// instead of fake location
						RsxRoot{
							node,
							location: None,
						}
					}
					RouteFunc::new(
						"get",
						#route_path_str,
						func
					)
				}},
			)?])
		}
		FuncFilesToRouteFuncs::new(map)
	}
}


impl<F: MapFuncFileToRoutes> FuncFilesToRouteFuncs<F> {
	pub fn new(func: F) -> Self { Self { func } }
}


impl<T: RsxPipelineTarget + AsRef<Vec<FuncFile>>, F: MapFuncFileToRoutes>
	RsxPipeline<T, Result<(T, Vec<RouteFuncTokens>)>>
	for FuncFilesToRouteFuncs<F>
{
	fn apply(self, func_files: T) -> Result<(T, Vec<RouteFuncTokens>)> {
		let route_funcs = func_files
			.as_ref()
			.iter()
			.map(|func_file| self.func.file_to_routes(func_file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		Ok((func_files, route_funcs))
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
			.pipe(FileGroupToFuncFiles::default())
			.unwrap()
			.pipe(FuncFilesToRouteFuncs::http_routes())
			.unwrap();
	}
}

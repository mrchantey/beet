use anyhow::Result;
use std::path::Path;
use sweet::prelude::*;
use syn::Block;
use syn::Visibility;


/// create the tokens for a specific route, it may contain
/// one or more http methods.
pub struct ParseRouteFile<'a> {
	pub path: &'a Path,
}

const HTTP_METHODS: [&str; 9] = [
	"get", "post", "put", "delete", "head", "options", "connect", "trace",
	"patch",
];

impl<'a> ParseRouteFile<'a> {
	/// reads a file and discovers all top level pub functions
	/// that match a http method
	pub fn parse(path: &Path) -> Result<Vec<Block>> {
		let file = ReadFile::to_string(path)?;
		let path_str = path.to_string_lossy();
		let items = syn::parse_file(&file)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(f) = item {
					match (&f.vis, f.sig.ident.to_string().as_ref()) {
						(Visibility::Public(_), ident) => {
							if HTTP_METHODS.contains(&ident) {
								return Some(f);
							}
						}
						_ => {}
					}
				}
				None
			})
			.map(|func| {
				let ident = &func.sig.ident;
				let method = func.sig.ident.to_string();

				syn::parse_quote! {
					{
						// some route thingie
						#[path=#path_str]
						mod route;
						(RouteInfo::new(#path_str,#method),route::#ident)
					}
				}
			})
			.collect::<Vec<_>>();
		Ok(items)
	}
}

use anyhow::Result;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Block;
use syn::Visibility;


/// create the tokens for a specific route, it may contain
/// one or more http methods.
pub struct ParseRouteFile {}

const HTTP_METHODS: [&str; 9] = [
	"get", "post", "put", "delete", "head", "options", "connect", "trace",
	"patch",
];

impl ParseRouteFile {
	/// reads a file and discovers all top level pub functions
	/// that match a http method
	pub fn parse(routes_dir: &str, path: PathBuf) -> Result<Vec<Block>> {
		let file = ReadFile::to_string(&path)?;

		// convert from ../routes/index.rs to index.rs
		let route_path = path.to_string_lossy();
		let route_relative_path =
			route_path.split(routes_dir).last().or_err()?;
		let route_file_path = route_relative_path
			.strip_prefix("/")
			.unwrap_or(&route_relative_path);
		// let route_mod_path = format!("{routes_dir}{route_relative_path}");
		let mut route_url_path = route_relative_path.replace(".rs", "");
		if route_url_path.ends_with("index") {
			route_url_path = route_url_path.replace("index", "");
		}

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
						#[path=#route_file_path]
						mod route;
						(RouteInfo::new(#route_url_path,#method),route::#ident)
					}
				}
			})
			.collect::<Vec<_>>();
		Ok(items)
	}
}

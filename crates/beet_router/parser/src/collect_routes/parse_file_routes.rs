use anyhow::Result;
use proc_macro2::TokenStream;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Visibility;


/// create the tokens for a specific route, ie `routes/index.rs`
/// any top level public function that matches a http method
/// will be parsed and returned as a block.
pub struct ParseFileRoutes;

const HTTP_METHODS: [&str; 9] = [
	"get", "post", "put", "delete", "head", "options", "connect", "trace",
	"patch",
];

impl ParseFileRoutes {
	/// reads a file and discovers all top level pub functions
	/// that match a http method
	pub fn parse(routes_dir: &str, path: PathBuf) -> Result<Vec<TokenStream>> {
		let file = ReadFile::to_string(&path)?;

		// convert from ../routes/index.rs to index.rs
		let route_path = path.to_string_lossy();
		let route_relative_path =
			route_path.split(routes_dir).last().or_err()?;
		// let route_mod_path = format!("{routes_dir}{route_relative_path}");
		let mut route_url_path = route_relative_path.replace(".rs", "");
		if route_url_path.ends_with("index") {
			route_url_path = route_url_path.replace("index", "");
		}

		let mod_name = path.file_stem().unwrap().to_string_lossy();
		let mod_name =
			syn::Ident::new(&mod_name, proc_macro2::Span::call_site());

		println!("{:#?}", mod_name);
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
						(RouteInfo::new(#route_url_path,#method),#mod_name::#ident)
				}
			})
			.collect::<Vec<_>>();
		Ok(items)
	}
}

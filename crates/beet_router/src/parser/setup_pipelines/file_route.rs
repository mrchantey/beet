use anyhow::Result;
use proc_macro2::TokenStream;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::ItemFn;
use syn::Visibility;


const HTTP_METHODS: [&str; 9] = [
	"get", "post", "put", "delete", "head", "options", "connect", "trace",
	"patch",
];

/// create the tokens for a specific route, ie `routes/index.rs`
/// any top level public function that matches a http method
/// will be parsed and returned as a block.
pub struct FileRoute {
	/// path to the file, ie foo/bar/index.rs
	file_path: PathBuf,
	/// url path for the route, ie /foo/bar
	url_path: String,
	/// all collected get, post etc methods
	methods: Vec<ItemFn>,
}


impl FileRoute {
	/// reads a file and discovers all top level pub functions
	/// that match a http method
	pub fn new(routes_dir: &str, path: PathBuf) -> Result<Self> {
		// convert from ../routes/index.rs to index.rs
		let file_path_str = path.to_string_lossy();
		let file_relative_path =
			file_path_str.split(routes_dir).last().or_err()?;
		// let route_mod_path = format!("{routes_dir}{route_relative_path}");
		let mut url_path = file_relative_path.replace(".rs", "");
		if url_path.ends_with("index") {
			url_path = url_path.replace("index", "");
			// remove trailing / from non root paths
			if url_path.len() > 1 {
				url_path.pop();
			}
		}

		let file = ReadFile::to_string(&path)?;

		let methods = syn::parse_file(&file)?
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
			.collect::<Vec<_>>();
		Ok(Self {
			file_path: path,
			url_path,
			methods,
		})
	}

	/// Create a file level pub const for the url path
	pub fn static_path(&self) -> TokenStream {
		let ident_str = self
			.file_path
			.file_stem()
			.unwrap()
			.to_string_lossy()
			.to_uppercase();
		let ident = syn::Ident::new(&ident_str, proc_macro2::Span::call_site());
		let url_path = &self.url_path;
		syn::parse_quote! {pub const #ident:&'static str = #url_path;}
	}

	/// Collect methods into a tuple vec of `(RouteInfo,func_ident)` tokens
	pub fn add_routes(&self) -> Vec<TokenStream> {
		let mod_name = self.file_path.file_stem().unwrap().to_string_lossy();
		let mod_name =
			syn::Ident::new(&mod_name, proc_macro2::Span::call_site());

		let url_path = &self.url_path;

		self.methods
			.iter()
			.map(|func| {
				let ident = &func.sig.ident;
				let method = func.sig.ident.to_string();

				syn::parse_quote! {
						(RouteInfo::new(#url_path,#method),#mod_name::#ident)
				}
			})
			.collect::<Vec<_>>()
	}
}

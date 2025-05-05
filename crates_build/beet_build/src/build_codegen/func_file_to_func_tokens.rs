use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use std::str::FromStr;
use sweet::prelude::*;
use syn::Ident;
use syn::Visibility;

/// For a given file group, collect all public functions of rust files.
#[derive(Debug, Clone)]
pub struct FuncFileToFuncTokens;

const HTTP_METHODS: [&str; 9] = [
	"get", "post", "put", "delete", "head", "options", "connect", "trace",
	"patch",
];

impl FuncFileToFuncTokens {
	pub fn parse(
		mod_ident: Ident,
		file_str: &str,
		canonical_path: AbsPathBuf,
		local_path: PathBuf,
	) -> Result<Vec<FuncTokens>> {
		let route_path = RoutePath::from_file_path(&local_path)?;

		let pub_funcs = syn::parse_file(&file_str)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(func) = item {
					match &func.vis {
						Visibility::Public(_) => {
							let sig_str = func.sig.ident.to_string();
							return Some((sig_str, func));
						}
						_ => {}
					}
				}
				None
			})
			.collect::<Vec<_>>();


		pub_funcs
			.iter()
			.filter_map(|(ident_str, item_fn)| {
				if HTTP_METHODS.iter().all(|m| m != &ident_str) {
					return None;
				}
				let frontmatter_ident = format!("{ident_str}_frontmatter");
				let frontmatter = match pub_funcs.iter().find(|(s, _)| {
					s == "frontmatter" || s == &frontmatter_ident
				}) {
					Some((_, frontmatter_ident)) => {
						syn::parse_quote!({
							#mod_ident::#frontmatter_ident()
						})
					}
					None => syn::parse_quote!({ Default::default() }),
				};

				Some(FuncTokens {
					mod_ident: mod_ident.clone(),
					mod_import: ModImport::Path,
					canonical_path: canonical_path.clone(),
					local_path: local_path.clone(),
					route_info: RouteInfo {
						path: route_path.clone(),
						// we just checked its a valid method
						method: HttpMethod::from_str(&ident_str).unwrap(),
					},
					frontmatter,
					item_fn: item_fn.clone(),
				})
			})
			.collect::<Vec<_>>()
			.xok()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let funcs = FileGroup::test_site_pages()
			.xpipe(FileGroupToFuncTokens::default())
			.unwrap();
		expect(funcs.len()).to_be(3);
		let file = funcs
			.iter()
			.find(|f| f.local_path.ends_with("docs/index.rs"))
			.unwrap();
		expect(&file.local_path.to_string_lossy()).to_be("docs/index.rs");
		expect(file.canonical_path.to_string_lossy().ends_with(
			"crates_rsx/beet_router/src/test_site/pages/docs/index.rs",
		))
		.to_be_true();
	}
}

use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use syn::Item;

#[derive(Debug, Clone)]
pub struct FuncTokensTreeToServerActions {
	pub codegen_file: CodegenFile,
	/// Collapse single child functions into their parent mod
	pub collapse_nodes: bool,
}

impl Default for FuncTokensTreeToServerActions {
	fn default() -> Self {
		Self {
			codegen_file: CodegenFile::default(),
			collapse_nodes: true,
		}
	}
}

impl Pipeline<FuncTokensTree, Result<()>> for FuncTokensTreeToServerActions {
	fn apply(mut self, tree: FuncTokensTree) -> Result<()> {
		let mod_tree = self.mod_tree(&tree);
		// let mod_imports = tree.into_group().item_mods(&self.codegen_file)?;
		// self.codegen_file.items.extend(mod_imports);
		self.codegen_file.add_item(mod_tree);
		self.codegen_file.build_and_write()?;
		Ok(())
	}
}


impl FuncTokensTreeToServerActions {
	pub fn new(codegen_file: CodegenFile) -> Self {
		Self {
			codegen_file,
			..Default::default()
		}
	}

	/// Create a tree of server actions
	fn mod_tree(&self, tree: &FuncTokensTree) -> Item {
		let item = self.mod_tree_inner(tree);
		if self.collapse_nodes {
			self.collapse_item(item)
		}else{
		item
		}
	}
	fn mod_tree_inner(&self, tree: &FuncTokensTree) -> Item {
		let ident = syn::Ident::new(&tree.name, proc_macro2::Span::call_site());
		let children =
			tree.children.iter().map(|child| self.mod_tree_inner(child));

		let items = tree
			.funcs
			.iter()
			.map(|tokens| tokens.xpipe(FuncTokensToServerActions::default()));

		syn::parse_quote! {
			#[allow(missing_docs)]
			pub mod #ident {
				#[allow(unused_imports)]
				use super::*;
				#(#items)*
				#(#children)*
			}
		}
	}



	/// If this item is a mod and its only child is a function, we can collapse it
	/// so that the function replaces the mod and its name becomes the mod name
	fn collapse_item(&self, item: Item) -> Item {
		if let Item::Mod(mut item_mod) = item {
			if let Some((_, ref mut items)) = item_mod.content {
				// the first is `use super::*;`
				if items.len() == 2 {
					if let Item::Fn(mut func) = items[1].clone() {
						func.sig.ident = item_mod.ident;
						return Item::Fn(func);
					}
				}
				// otherwise map all children
				*items = items
					.drain(..)
					.map(|item| self.collapse_item(item.clone()))
					.collect();
			}
			Item::Mod(item_mod)
		} else {
			item
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn simple() {
		FuncTokensTreeToServerActions::default()
			.mod_tree(&vec![FuncTokens::simple_get("bazz.rs")].into())
			.xmap(|item| item.to_token_stream().to_string())
			.xmap(expect)
			.to_be(quote! {
				#[allow (missing_docs)]
				pub mod root {
					#[allow(unused_imports)]
					use super::*;	
					pub async fn bazz() -> Result<(), ServerActionError> {
						CallServerAction::request_no_data(RouteInfo::new("/bazz", HttpMethod::Get)).await
					}
				}
			}.to_string());
	}

	#[test]
	fn correct_tree_structure() {
		FuncTokensTreeToServerActions::default()
			.mod_tree(&vec![
				FuncTokens::simple_with_func("bazz.rs", syn::parse_quote!(fn get() {})),
				FuncTokens::simple_with_func("foo/bar.rs", syn::parse_quote!(fn get() {})),
				FuncTokens::simple_with_func("foo/boo.rs", syn::parse_quote!(fn get() {})),
				FuncTokens::simple_with_func("foo/boo.rs", syn::parse_quote!(fn post() {})),
				FuncTokens::simple_with_func("foo/bing/bong.rs", syn::parse_quote!(fn post() {})),
			]
			.into())
			.xmap(|item| item.to_token_stream().to_string())
			.xmap(expect)
			.to_be(quote! {
				#[allow(missing_docs)]
				pub mod root {
					#[allow(unused_imports)]
					use super::*;
	
						pub async fn bazz() -> Result<(), ServerActionError> {
								CallServerAction::request_no_data(RouteInfo::new("/bazz", HttpMethod::Get)).await
						}

						#[allow(missing_docs)]
						pub mod foo {
							#[allow(unused_imports)]
							use super::*;
					
								pub async fn bar() -> Result<(), ServerActionError> {
										CallServerAction::request_no_data(RouteInfo::new("/foo/bar", HttpMethod::Get)).await
								}

								#[allow(missing_docs)]
								pub mod boo {
									#[allow(unused_imports)]
									use super::*;
									
									pub async fn get() -> Result<(), ServerActionError> {
										CallServerAction::request_no_data(RouteInfo::new("/foo/boo", HttpMethod::Get)).await
									}

									pub async fn post() -> Result<(), ServerActionError> {
										CallServerAction::request_no_data(RouteInfo::new("/foo/boo", HttpMethod::Post)).await
									}
								}
								#[allow(missing_docs)]
								pub mod bing {			
									#[allow(unused_imports)]
									use super::*;
									pub async fn bong() -> Result<(), ServerActionError> {
											CallServerAction::request_no_data(RouteInfo::new("/foo/bing/bong", HttpMethod::Post)).await
									}
								}
						}
				}
			}.to_string())
	}
}

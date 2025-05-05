use crate::prelude::*;
use anyhow::Result;
use std::ops::Deref;
use std::ops::DerefMut;
use sweet::prelude::VecExt;
use syn::Expr;
use syn::Item;
use syn::ItemFn;
use syn::Type;


/// A group of [`FuncTokens`] and a set of common helpers.
pub struct FuncTokensGroup {
	/// A group of [`FuncTokens`] that are all the same type.
	pub funcs: Vec<FuncTokens>,
}

impl Deref for FuncTokensGroup {
	type Target = [FuncTokens];
	fn deref(&self) -> &Self::Target { &self.funcs }
}

impl DerefMut for FuncTokensGroup {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.funcs }
}


impl AsRef<FuncTokensGroup> for FuncTokensGroup {
	fn as_ref(&self) -> &FuncTokensGroup { self }
}


impl FuncTokensGroup {
	pub fn new(funcs: Vec<FuncTokens>) -> Self { FuncTokensGroup { funcs } }


	/// Creates an `ItemMod` for each [`FuncTokens`] in the group,
	/// required for calling the [`FuncTokens::item_fn`]
	/// ## Errors
	///
	/// Errors if a relative path between the [`FuncTokens::canonical_path`]
	/// and the `out_dir` cannot be created.
	pub fn item_mods(&self, codegen_file: &CodegenFile) -> Result<Vec<Item>> {
		self.funcs
			.iter()
			.map(|func| func.item_mod(codegen_file).map(|item| item.into()))
			.collect::<Result<Vec<Item>>>()
	}


	pub fn collect_func(
		&self,
		output: &Type,
		map_func: impl Fn(&FuncTokens) -> Expr,
	) -> ItemFn {
		let funcs = self.funcs.iter().map(map_func);

		syn::parse_quote! {
			/// Collect all functions from their files as defined in the [`AppConfig`]
			#[allow(dead_code)]
			pub fn collect() -> Vec<#output> {
				vec![#(#funcs),*]
			}
		}
	}

	pub fn into_tree(self) -> FuncTokensTree { self.into() }
}

impl Into<FuncTokensTree> for FuncTokensGroup {
	fn into(self) -> FuncTokensTree {
		let mut this = FuncTokensTree::new("root");
		for func in self.funcs {
			// 	// should be ancestors
			// 	// let parts = ;
			let mut current = &mut this;
			for component in func.route_info.path.components() {
				match component {
					std::path::Component::Normal(os_str)
						if let Some(str) = os_str.to_str() =>
					{
						current = VecExt::entry_or_insert_with(
							&mut current.children,
							|child| child.name == str,
							|| FuncTokensTree::new(str),
						);
					}
					_ => {} // std::path::Component::Prefix(prefix_component) => todo!(),
					        // std::path::Component::RootDir => todo!(),
					        // std::path::Component::CurDir => todo!(),
					        // std::path::Component::ParentDir => todo!(),
				}
			}
			current.funcs.push(func);
		}
		this
	}
}

impl From<Vec<FuncTokens>> for FuncTokensGroup {
	fn from(value: Vec<FuncTokens>) -> Self { FuncTokensGroup::new(value) }
}

impl From<FuncTokens> for FuncTokensGroup {
	fn from(value: FuncTokens) -> Self { FuncTokensGroup::new(vec![value]) }
}

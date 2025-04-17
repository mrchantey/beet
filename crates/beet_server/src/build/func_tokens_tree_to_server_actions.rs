use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use proc_macro2::TokenStream;
use syn::Item;

#[derive(Debug, Default, Clone)]
pub struct FuncTokensTreeToServerActions {
	pub codegen_file: CodegenFile,
}



impl Pipeline<FuncTokensTree, Result<()>> for FuncTokensTreeToServerActions {
	fn apply(mut self, tree: FuncTokensTree) -> Result<()> {
		let mod_tree = self.mod_tree(&tree);
		let mod_imports = tree.into_group().item_mods(&self.codegen_file)?;
		self.codegen_file.items.extend(mod_imports);
		self.codegen_file.add_item(mod_tree);
		self.codegen_file.build_and_write()?;
		Ok(())
	}
}


impl FuncTokensTreeToServerActions {
	pub fn new(codegen_file: CodegenFile) -> Self { Self { codegen_file } }

	fn mod_tree(&self, tree: &FuncTokensTree) -> Item {
		tree.mod_tree(|node| {
			node.value
				.as_ref()
				.map(|tokens| {
					tokens.xpipe(FuncTokensToServerActions::default()).into()
				})
				.unwrap_or(Item::Verbatim(TokenStream::new()))
		})
	}
}

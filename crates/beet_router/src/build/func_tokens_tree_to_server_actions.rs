use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use syn::Item;

#[derive(Debug, Default, Clone)]
pub struct FuncTokensTreeToServerActions {
	pub codegen_file: CodegenFile,
}



impl Pipeline<FuncTokensTree, Result<()>> for FuncTokensTreeToServerActions {
	fn apply(mut self, tree: FuncTokensTree) -> Result<()> {
		let mod_tree = self.mod_tree(&tree);
		// let collect_func = self.collect_func(&tree);
		self.codegen_file.add_item(mod_tree);
		self.codegen_file.build_and_write()?;
		Ok(())
	}
}


impl FuncTokensTreeToServerActions {
	fn mod_tree(&self, tree: &FuncTokensTree) -> Item {
		tree.mod_tree(|node| todo!())
	}
}

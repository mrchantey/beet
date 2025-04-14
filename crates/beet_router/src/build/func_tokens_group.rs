use crate::prelude::*;




/// A group of [`FuncTokens`] for which a type has been
/// determined, and so is ready for codegen steps like creating
/// a `collect` function.
pub struct FuncTokensGroup {
	/// The return type of each [`FuncTokens::func`] block.
	pub func_type: syn::Type,
	/// A group of [`FuncTokens`] that are all the same type.
	pub funcs: Vec<FuncTokens>,
}


impl AsRef<FuncTokensGroup> for FuncTokensGroup {
	fn as_ref(&self) -> &FuncTokensGroup { self }
}

use crate::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


/// Placed at the root of a parsed template macro, with an index
/// indicating the position of the macro in the source file visited in order
/// of *tokens* not an actual tree.
/// This type only tracks the start of a macro, so only a change in location,
/// not size, will change the hash.
/// Combining this with [`ExprIdx`] we can uniquely identify
/// a template macro in a file, and the order of expressions inside it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct MacroIdx {
	/// The source file containing the template.
	pub file: WsPathBuf,
	/// The index of the template in the file.
	/// - For md and rsx files this is always 0 as they only have one template.
	/// - For rust files this is the top-down appearance of the `rsx!` macro.
	pub start: LineCol,
}
impl MacroIdx {
	/// Create a new [`TemplateKey`] from a file and index.
	pub fn new(file: WsPathBuf, start: LineCol) -> Self { Self { file, start } }
}

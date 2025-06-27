use crate::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


/// Placed at the root of a parsed template macro, with a [`LineCol`] representing
/// the start of the macro in the source file. Only a change in start [`LineCol`],
/// not internal size or end [`LineCol`], will change the hash.
/// Combining this with [`ExprIdx`] we can uniquely identify
/// a template macro in a file, and the order of expressions inside it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct MacroIdx {
	/// The source file containing the template.
	pub file: WsPathBuf,
	/// The index of the template in the file.
	/// - For md and rsx files this is always be [`LineCol::default()`] as they are one big 'macro'.
	/// - For rust files this is the top-down appearance of the `rsx!` macro.
	pub start: LineCol,
}
impl MacroIdx {
	/// Create a new [`TemplateKey`] from a file and index.
	pub fn new(file: WsPathBuf, start: LineCol) -> Self { Self { file, start } }
	#[cfg(feature = "tokens")]
	pub fn new_from_tokens(
		file: WsPathBuf,
		token: &proc_macro2::TokenStream,
	) -> Self {
		use syn::spanned::Spanned;
		Self {
			file,
			start: token.span().start().into(),
		}
	}
}



/// Static nodes are created by statically analyzing a file,
/// so they should not be rendered directly, and only used for template reloading.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StaticNodeRoot;

/// Added to non-static entities with a [`MacroIdx`], indicating they have
/// had the [`StaticNodeRoot`] applied.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct ResolvedRoot;

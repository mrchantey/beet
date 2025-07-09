//! Types associated with the root node of a tree in beet.
use crate::as_beet::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_utils::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;


/// The root of a beet node tree, which is guaranteed to be
/// an internally created [`FragmentNode`], even if the root of an `rsx!`
/// macro is also a fragment.
/// This allows for some very important functionality:
///
/// - Rearranging a Html Document while maintaining the same root node.
/// - The root of the macro in [`RsxSnippetRoot`] can safely change between
///   a fragment and not.
/// - Avoiding [`ExprIdx`], [`ChildOf`], etc collisions when applying
///  [`StaticNodeRoot`] to an [`InstanceRoot`].
///
/// The [`BeetRoot`] can have multiple children, for example
/// `rsx!{<br/><br/>}`
///
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[require(FragmentNode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct BeetRoot;


/// Static nodes are created by statically analyzing a file,
/// so they should not be rendered directly, and only used for template reloading.
///
/// These have some important differences from an [`InstanceRoot`]:
/// - Template nodes actually have [`Attributes`], instead of applying them as props.
/// - They contain no evaluated [`NodeExpr`], which can only be evaluated by
///   running code. They do however contain a matching [`ExprIdx`] for each
///  	[`NodeExpr`].
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[require(BeetRoot)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RsxSnippetRoot;



/// An instantiated node tree, ie the output of an `rsx!` macro.
/// The [`OnSpawnTemplate`] systems may or may not have been evaluated yet,
/// which can be determined by the presence of a [`ResolvedRoot`].
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[require(BeetRoot)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct InstanceRoot;

/// Added to non-static entities with a [`MacroIdx`], indicating they have
/// had the [`StaticNodeRoot`] applied.
/// The [`OnSpawnTemplate`] systems have been evaluated, ie this node will not
/// have any, and instead contain the resolved children and templates.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[require(BeetRoot, InstanceRoot)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ResolvedRoot;



/// Placed at the root of each [`RsxSnippetRoot`] and [`InstanceRoot`], with a [`LineCol`] representing
/// the start of the macro in the source file. Only a change in start [`LineCol`],
/// not internal size or end [`LineCol`], will change the hash.
/// Combining this with [`ExprIdx`] we can uniquely identify
/// a template macro in a file, and the order of expressions inside it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
#[require(BeetRoot)]
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
	/// Convenience for using the `file!` and friends macros.
	/// ## Example
	/// ```rust
	/// use beet_common::prelude::*;
	/// let idx = MacroIdx::new_file_line_col(file!(), line!(), column!());
	/// ```
	pub fn new_file_line_col(file: &str, line: u32, col: u32) -> Self {
		Self {
			file: WsPathBuf::new(file),
			start: LineCol::new(line, col),
		}
	}

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

/// Print the closest [`MacroIdx`] ancestor of the entity,
#[derive(SystemParam)]
pub struct NodeLocation<'w, 's> {
	parents: Query<'w, 's, &'static ChildOf>,
	roots: Query<'w, 's, &'static MacroIdx>,
}
impl NodeLocation<'_, '_> {
	/// Get the [`MacroIdx`] of the closest ancestor of the entity.
	pub fn get_macro_idx(&self, entity: Entity) -> Option<&MacroIdx> {
		self.parents
			.iter_ancestors_inclusive(entity)
			.find_map(|e| self.roots.get(e).ok())
	}
	/// Get the [`MacroIdx`] of the closest ancestor of the entity.
	pub fn stringify(&self, entity: Entity) -> String {
		self.get_macro_idx(entity)
			.map(|idx| idx.to_string())
			.unwrap_or_else(|| format!("Entity without location: {entity:?}"))
	}
}


impl std::fmt::Display for MacroIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.file, self.start)
	}
}

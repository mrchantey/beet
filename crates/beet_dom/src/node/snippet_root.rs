//! Types associated with the root node of a tree in beet.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Specify types for variadic functions like TokenizeComponent
pub type RootComponents = (
	SnippetRoot,
	BeetRoot,
	StaticRoot,
	InstanceRoot,
	ExprIdx,
	DomIdx,
	RequiresDomIdx,
);


/// Used to mark the root of a resolved tree:
/// - a [`HtmlDocument`]
/// - a [`ClientIslandRoot`]
/// - when rendering a [`HtmlFragment`]
/// The root needs to be explicitly marked so we can know the entrypoint of a resolved tree
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct BeetRoot;


/// Placed at the root of each [`StaticRoot`] and [`InstanceRoot`], with a [`LineCol`] representing
/// the start of the macro in the source file. Only a change in start [`LineCol`],
/// not internal size or end [`LineCol`], will change the hash.
/// Combining this with [`ExprIdx`] we can uniquely identify
/// a template macro in a file, and the order of expressions inside it.
///
/// ## Structure
/// This is guaranteed to be a [`FragmentNode`].
/// If the root of a macro is also a fragment that will be a nested fragment,
/// allowing for some very important functionality:
///
/// - Rearranging a Html Document while maintaining the same root node.
/// - The root of the macro in [`StaticRoot`] can safely change between
///   a fragment and not.
/// - Avoiding [`ExprIdx`], [`ChildOf`], etc collisions when applying
///  [`StaticRoot`] to an [`InstanceRoot`].
///
/// The [`SnippetRoot`] can have multiple children, for example
/// `rsx!{<br/><br/>}`
///
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
#[require(FragmentNode)]
pub struct SnippetRoot {
	/// The source file containing the snippet.
	pub file: WsPathBuf,
	/// The index of the template in the file.
	/// - For md and rsx files this is always be [`LineCol::default()`] as they are one big 'macro'.
	/// - For rust files this is the top-down appearance of the `rsx!` macro.
	pub start: LineCol,
}

impl std::fmt::Display for SnippetRoot {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.file, self.start)
	}
}

impl SnippetRoot {
	/// Create a new [`SnippetRoot`] from a file and index.
	pub fn new(file: WsPathBuf, start: LineCol) -> Self { Self { file, start } }
	/// Convenience for using the `file!` and friends macros.
	/// ## Example
	/// ```rust
	/// use beet_dom::prelude::*;
	/// let idx = SnippetRoot::new_file_line_col(file!(), line!(), column!());
	/// ```
	pub fn new_file_line_col(file: &str, line: u32, col: u32) -> Self {
		Self {
			file: WsPathBuf::new(file),
			start: LineCol::new(line, col),
		}
	}

	/// Creates a new [`SnippetRoot`] from a token stream.
	///
	/// Extracts the start position from the token stream's span.
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


/// Indicates this snippet was created by statically analyzing a file or token stream,
/// meaning it will not have access to any of they dynamic content, instead storing them
/// in a [`NodeExpr`]. These are used for snippet reloading.
///
/// These have some important differences from an [`InstanceRoot`]:
/// - Each [`TemplateNode`] has [`Attributes`], instead of applying them as props.
/// - They contain [`NodeExpr`], which can only be evaluated by
///   running code. They do however contain a matching [`ExprIdx`] for each
///  	[`NodeExpr`].
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[require(SnippetRoot)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct StaticRoot;

/// An instantiated node tree, ie the output of an `rsx!` macro.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[require(SnippetRoot)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct InstanceRoot;

/// Utility for getting the closest [`SnippetRoot`] ancestor of the entity,
///
#[derive(SystemParam)]
pub struct NodeLocation<'w, 's> {
	parents: Query<'w, 's, &'static ChildOf>,
	roots: Query<'w, 's, &'static SnippetRoot>,
}
impl NodeLocation<'_, '_> {
	/// Get the [`SnippetRoot`] of the closest ancestor of the entity.
	pub fn get_snippet_root(&self, entity: Entity) -> Option<&SnippetRoot> {
		self.parents
			.iter_ancestors_inclusive(entity)
			.find_map(|e| self.roots.get(e).ok())
	}
	/// Get the [`SnippetRoot`] of the closest ancestor of the entity.
	pub fn stringify(&self, entity: Entity) -> String {
		self.get_snippet_root(entity)
			.map(|idx| idx.to_string())
			.unwrap_or_else(|| format!("Entity without location: {entity:?}"))
	}
}

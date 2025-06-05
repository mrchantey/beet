use crate::node_tokens::resolve_attribute_values;
use crate::prelude::*;
// use beet_common::prelude::*;
// use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::TokenStream;
// use quote::quote;
use send_wrapper::SendWrapper;
// use sweet::prelude::PipelineTarget;
// use syn::Expr;


/// Marker component to be swapped out for a [`IrTokens`],
/// containing the rust tokens for the node.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[component(storage = "SparseSet")]
pub struct GetExprTreeTokens {
	/// whether parsing errors should be excluded from the output.
	exclude_errors: bool,
}

/// A [`TokenStream`] representing a bevy bundle in the form of an
/// Intermediate Representation of some hierarchy. For example,
/// an [`ItemOf<BlockNode,syn::Expr>`] is exported as-is.
#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct ExprTreeTokens(pub SendWrapper<TokenStream>);
impl ExprTreeTokens {
	pub fn new(value: TokenStream) -> Self { Self(SendWrapper::new(value)) }
	pub fn take(self) -> TokenStream { self.0.take() }
}



pub fn tokenize_expr_tree_plugin(app: &mut App) {
	app.add_systems(
		Update,
		tokenize_expr_tree
			// i *think* we want the resolved combinator attribute expressions,
			// but can change this to 'before' if we dont
			.after(resolve_attribute_values)
			.in_set(ExportNodesStep),
	);
}


fn tokenize_expr_tree() {}


// /// recursively visit children and collect into a [`TokenStream`].
// /// We use a custom [`SystemParam`] for the traversal, its more of
// /// a 'map' function than an 'iter', as we need to resolve children
// /// and then wrap them as `children![]` in parents.
// #[derive(SystemParam)]
// struct IrBuilder<'w, 's> {
// 	children: TokenizeRelated<'w, 's, Children>,
// 	// children: Query<'w, 's, &'static Children>,
// 	block_node_exprs:
// 		Query<'w, 's, &'static ItemOf<BlockNode, SendWrapper<Expr>>>,
// 	combinators: Query<'w, 's, &'static CombinatorExpr>,
// 	rsx_nodes: TokenizeRsxNode<'w, 's>,
// 	rsx_directives: TokenizeRsxDirectives<'w, 's>,
// 	web_nodes: TokenizeWebNodes<'w, 's>,
// 	web_directives: TokenizeWebDirectives<'w, 's>,
// 	node_attributes: TokenizeAttributes<'w, 's>,
// }

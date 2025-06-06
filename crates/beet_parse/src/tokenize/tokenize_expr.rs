use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Expr;


pub fn tokenize_block_node_exprs(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	if let Some(block) = world
		.entity(entity)
		.get::<ItemOf<BlockNode, SendWrapper<Expr>>>()
	{
		let block = &***block;
		Ok(Some(quote! {#block.into_node_bundle()}))
	} else {
		Ok(None)
	}
}
/// push combinators for nodes, attributes are handled by CollectNodeAttributes
pub fn tokenize_combinator_exprs(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	if let Some(combinator) = world.entity(entity).get::<CombinatorExpr>() {
		let mut expr = String::new();
		for item in combinator.iter() {
			match item {
				CombinatorExprPartial::Tokens(tokens) => {
					expr.push_str(tokens);
				}
				CombinatorExprPartial::Element(entity) => {
					let tokens = tokenize_bundle(world, *entity)?;
					expr.push_str(&tokens.to_string());
				}
			}
		}
		// combinator removes braces so we put them back
		let expr = format!("{{{}}}", expr);
		let expr_tokens = syn::parse_str::<TokenStream>(&expr)?;
		Ok(Some(expr_tokens))
	} else {
		Ok(None)
	}
}

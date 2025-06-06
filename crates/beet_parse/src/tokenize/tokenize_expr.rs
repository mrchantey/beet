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

pub fn tokenize_combinator_exprs_to_bundle(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	tokenize_combinator_exprs(world, entity, tokenize_bundle)
}

pub fn tokenize_combinator_exprs_to_node_tree(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	tokenize_combinator_exprs(world, entity, tokenize_node_tree)
}

/// push combinators for nodes, attributes are handled by tokenize_attributes
fn tokenize_combinator_exprs(
	world: &World,
	entity: Entity,
	map_child: impl Fn(&World, Entity) -> Result<TokenStream>,
) -> Result<Option<TokenStream>> {
	if let Some(combinator) = world.entity(entity).get::<CombinatorExpr>() {
		let mut expr = String::new();
		for item in combinator.iter() {
			match item {
				CombinatorExprPartial::Tokens(tokens) => {
					expr.push_str(tokens);
				}
				CombinatorExprPartial::Element(entity) => {
					let tokens = map_child(world, *entity)?;
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

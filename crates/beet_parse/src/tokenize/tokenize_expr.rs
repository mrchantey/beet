use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;



pub fn tokenize_combinator_exprs(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	tokenize_combinator_exprs_inner(world, entity, super::tokenize_bundle)
}

pub fn tokenize_combinator_exprs_tokens(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	tokenize_combinator_exprs_inner(world, entity, tokenize_bundle_tokens)
}

/// push combinators for nodes, attributes are handled by tokenize_attributes
fn tokenize_combinator_exprs_inner(
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
		let expr = format!("{{{expr}}}");
		let expr_tokens = syn::parse_str::<TokenStream>(&expr)?;
		Ok(Some(expr_tokens))
	} else {
		Ok(None)
	}
}

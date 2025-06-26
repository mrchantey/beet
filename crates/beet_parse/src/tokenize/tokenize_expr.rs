use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;





/// push combinators for nodes, attributes are handled by tokenize_attributes
pub fn tokenize_combinator_exprs_mapped(
	world: &World,
	entity: Entity,
	map_child: impl Fn(&World, Entity) -> Result<TokenStream>,
) -> Result<Option<NodeExpr>> {
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
		let expr_tokens = syn::parse_str(&expr)?;
		Ok(Some(NodeExpr::new_block(expr_tokens)))
	} else {
		Ok(None)
	}
}

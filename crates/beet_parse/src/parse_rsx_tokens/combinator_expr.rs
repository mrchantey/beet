use crate::prelude::*;
use beet_core::prelude::*;
use proc_macro2::TokenStream;


/// The partially parsed equivalent of a [`RsxParsedExpression`](beet_rsx_combinator::types::RsxParsedExpression),
/// eventually collected into a [`NodeExpr`]
///
/// [`beet_rsx_combinator`] is very different from macro/tokens based parsers.
/// A fundamental concept is support for mixed expressions `let foo = <div/>;`
/// which means we need to parse `let foo =` seperately from `<div/>`. So the
/// element is added in a similar way to [`rstml`] so that we can still
/// apply scoped styles etc, but the hierarchy is not exactly correct, as
/// elements are parsed in the order they are defined not applied.
/// It can later be combined into a single expression
/// `let foo = (NodeTag("div"),ElementNode{self_closing=true});`
///
#[derive(Default, Clone, Deref, DerefMut, Component, ToTokens)]
pub struct CombinatorExpr(pub Vec<CombinatorExprPartial>);

/// A section of a [`CombinatorExpr`],
/// a 1:1 mapping from [`RsxTokensOrElement`](beet_rsx_combinator::types::RsxTokensOrElement)
#[derive(Clone, ToTokens)]
pub enum CombinatorExprPartial {
	/// partial expressions must be a string as it may not be a valid
	/// TokenTree at this stage, for instance {let foo = <bar/>} will be split into
	/// `{let foo =` + `<bar/>` + `}`, unclosed braces are not a valid [`TokenStream`]
	Tokens(String),
	/// Reference to the entity containing the [`NodeTag`], [`ElementNode`] etc
	Element(Entity),
}



/// Iterate over all [`CombinatorExpr`] and collapse them,
pub fn collapse_combinator_exprs(world: &mut World) -> Result {
	let mut query = world.query_filtered::<Entity, With<CombinatorExpr>>();

	// just do an iter, it shouldnt matter if nested ones are resolved first
	while let Some(entity) = query.iter(world).next() {
		if let Some(expr) =
			// here we are only tokenizing with tokenize_bundle,
			// if at some point we want to tokenize_bundle_tokens for combinator exprs
			// we should introduce a marker component to the root, and decide which to use here
			tokenize_combinator_exprs_mapped(
				world,
				entity,
				tokenize_bundle,
			)? {
			world
				.entity_mut(entity)
				.remove::<CombinatorExpr>()
				.insert(expr);
		}
	}
	Ok(())
}


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
		let expr_tokens =
			syn::parse_str(&format!("{{{expr}}}")).map_err(|e| {
				bevyhow!(
					"Failed to parse combinator expression.\nInput: {expr}\nError: {e}"
				)
			})?;
		Ok(Some(NodeExpr::new_block(expr_tokens)))
	} else {
		Ok(None)
	}
}


#[allow(unused)]
fn tokenize_combinator_exprs(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) =
		tokenize_combinator_exprs_mapped(world, entity, tokenize_bundle)?
	{
		items.push(expr.insert_deferred());
	}
	Ok(())
}

#[allow(unused)]
fn tokenize_combinator_exprs_tokens(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) =
		tokenize_combinator_exprs_mapped(world, entity, tokenize_bundle_tokens)?
	{
		items.push(expr.self_token_stream());
	}
	Ok(())
}

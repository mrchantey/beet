use beet_common::as_beet::*;
use bevy::prelude::*;



/// The partially parsed equivalent of a [`RsxParsedExpression`](beet_rsx_combinator::types::RsxParsedExpression).
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
#[derive(Default, Component, Deref, DerefMut, ToTokens)]
pub struct CombinatorExpr(pub Vec<CombinatorExprPartial>);

/// A section of a [`CombinatorExpr`],
/// a 1:1 mapping from [`RsxTokensOrElement`](beet_rsx_combinator::types::RsxTokensOrElement)
#[derive(ToTokens)]
pub enum CombinatorExprPartial {
	/// partial expressions must be a string as it may not be a valid
	/// TokenTree at this stage, for instance {let foo = <bar/>} will be split into
	/// `{let foo =` + `<bar/>` + `}`, unclosed braces are not a valid [`TokenStream`]
	Tokens(String),
	/// Reference to the entity containing the [`NodeTag`], [`ElementNode`] etc
	Element(Entity),
}

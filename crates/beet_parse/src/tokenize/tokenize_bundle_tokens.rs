use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use send_wrapper::SendWrapper;
use beet_utils::prelude::*;
use syn::Expr;


/// Create a [`TokenStream`] of a [`Bundle`] that represents the *tokenized*
/// tree of nodes for the given [`Entity`], as opposed to the *finalized* tree,
#[rustfmt::skip]
pub fn tokenize_bundle_tokens(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
		// The root is not an actual node, so we flatten the children if its 1, or
	// convert to a fragment.
	flatten_fragment(world, entity, tokenize_bundle_tokens_no_flatten)
}
#[rustfmt::skip]
pub(super) fn tokenize_bundle_tokens_no_flatten(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let mut items = Vec::new();
	tokenize_rsx_nodes(world, &mut items, entity)?;
	tokenize_rsx_directives(world, &mut items, entity)?;
	tokenize_web_nodes(world, &mut items, entity)?;
	tokenize_web_directives(world, &mut items, entity)?;
	tokenize_block_node_exprs(world, &mut items, entity)?;
	tokenize_combinator_exprs_tokens(world, entity)?.map(|i|items.push(i));
	tokenize_related::<Attributes>(world, &mut items, entity, tokenize_attribute_tokens)?;
	tokenize_related::<Children>(world, &mut items, entity, tokenize_bundle_tokens_no_flatten)?;

	items.xmap(unbounded_bundle).xok()
}

fn tokenize_block_node_exprs(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) = world.entity(entity).get::<ItemOf::<BlockNode,SendWrapper<Expr>>>() {
		let block_node = expr.self_token_stream();
		items.push(block_node);
	}

	Ok(())
}
fn tokenize_attribute_tokens(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let entity = world.entity(entity);
	let mut items = Vec::new();
	if let Some(attr_expr) = entity.get::<AttributeExpr>() {
		items.push(attr_expr.self_token_stream());
	}
	if let Some(attr_key) = entity.get::<AttributeKeyExpr>() {
		items.push(attr_key.self_token_stream());
	}
	if let Some(attr_val) = entity.get::<AttributeValueExpr>() {
		items.push(attr_val.self_token_stream());
	}
let attr_entity = entity.id();
	if let Some(attr) =
				tokenize_combinator_exprs_tokens(world, attr_entity)?
	{
				items.push(attr);
	}

	unbounded_bundle(items).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;
	use beet_utils::prelude::*;

	fn parse_rstml(tokens: TokenStream) -> Matcher<String> {
		tokenize_rstml_tokens(tokens, WsPathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	fn parse_combinator(tokens: &str) -> Matcher<String> {
		tokenize_combinator_tokens(tokens, WsPathBuf::new(file!()))
				.unwrap()
				.to_string()
				.replace(" ", "")
				.chars()
				.skip(33)
				.collect::<String>()
				.chars()
				.rev()
				.skip(4)
				.collect::<String>()
				.chars()
				.rev()
				.collect::<String>()
				.xpect()
	}

	#[test]
	fn tag_only() {
		parse_rstml(quote! {<br/>}).to_be(
			quote! {
				(
					NodeTag(String::from("br")),
					ElementNode { self_closing: true }
				)
			}
			.to_string(),
		);
		parse_rstml(quote! {<Foo/>}).to_be(
			quote! {
				(
					NodeTag(String::from("Foo")),
					FragmentNode,
					TemplateNode
				)
			}
			.to_string(),
		);
	}
	#[test]
	fn attributes() {
		parse_rstml(quote! {
			<br 
				hidden
				class="foo"
				some_key={bar}
				onmousemove="some_js_func"
				onclick={|_: Trigger<OnClick>| {}}
			/>}).to_be(
			quote! {
				(
					NodeTag(String::from("br")),
					ElementNode { self_closing: true },
					related!{Attributes[
						AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("hidden"))),
						(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("class"))), 			AttributeValueExpr(SendWrapper::new(syn::parse_quote!("foo")))),
						(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("some_key"))) , 	AttributeValueExpr(SendWrapper::new(syn::parse_quote!({ bar })))),
						(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("onmousemove"))), AttributeValueExpr(SendWrapper::new(syn::parse_quote!("some_js_func")))),
						(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("onclick"))), 		AttributeValueExpr(SendWrapper::new(syn::parse_quote!({ |_: Trigger<OnClick>| {} }))))
					]}
				)
			}
			.to_string(),
		);
	}
	#[test]
	fn block_node() {
		parse_rstml(quote! {<div>{7}</div>}).to_be(
			quote! {(
							NodeTag(String::from("div")),
							ElementNode { self_closing: false },
							related!{Children[(
								BlockNode,
								ItemOf::<beet_common::node::rsx_nodes::BlockNode, send_wrapper::SendWrapper<syn::expr::Expr> > {
									value: SendWrapper::new(syn::parse_quote!({ 7 })),
									phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::BlockNode>
								}
							)]}
						)
			}
			.to_string(),
		);
	}
	#[test]
	fn combinator_simple() {
		parse_combinator("<br/>").to_be(
			quote! {
				(
					NodeTag(String::from("br")),
					ElementNode { self_closing: true }
				)
			}
			.to_string().replace(" ", ""),
		);
	}
	#[test]
	fn combinator_siblings() {
		tokenize_combinator_tokens("<br/><br/>", WsPathBuf::new(file!()))
				.unwrap()
				.to_string()
				.xpect().to_be(
			quote! {{
				(
					FragmentNode,
					related!{Children[(
						NodeTag(String::from("br")),
						ElementNode { self_closing: true }
					),
					(
						NodeTag(String::from("br")),
						ElementNode { self_closing: true }
					)]}
				)
			}}
			.to_string(),
		);
	}

	#[test]
	fn combinator() {
		parse_combinator(r#"
			<br 
				hidden
				class="foo"
				onmousemove="some_js_func"
				onclick={|_: Trigger<OnClick>| {}}
			/>
		"#).to_be(
			quote! {
					(
						NodeTag (String :: from ("br")), 
						ElementNode { self_closing : true }, 
						related!{Attributes[
							AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("hidden"))), 
							(AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("class"))), AttributeValueExpr (SendWrapper::new(syn::parse_quote!("foo")))), 
							(AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("onmousemove"))), AttributeValueExpr (SendWrapper::new(syn::parse_quote!("some_js_func")))), 
							(AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("onclick"))), {|_:Trigger<OnClick>| { } })
						]}
					)
			}
			.to_string().replace(" ", ""),
		);
	}
	#[test]
	fn nested_combinator() {
		parse_combinator(r#"<br 
				foo={
					let class = "bar";
					<div class={class}/>
				}
			/>"#).to_be(
			quote! {
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true },
							related!{Attributes[(
								AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("foo"))),
								{
									let class = "bar";
									(
										NodeTag(String::from("div")),
										ElementNode { self_closing: true },
										related!{Attributes[(
											AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("class"))),
											{ class }
										)]}
									)
								}
							)]}
						)
					}
			.to_string().replace(" ", "")
		);
	}


}

use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use send_wrapper::SendWrapper;
use beet_utils::prelude::*;
use syn::Expr;

#[rustfmt::skip]
pub fn tokenize_node_tree(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let mut items = Vec::new();
	tokenize_rsx_nodes(world, &mut items, entity)?;
	tokenize_rsx_directives(world, &mut items, entity)?;
	tokenize_web_nodes(world, &mut items, entity)?;
	tokenize_web_directives(world, &mut items, entity)?;
	tokenize_block_node_exprs(world, &mut items, entity)?;
	tokenize_combinator_exprs_to_node_tree(world, entity)?.map(|i|items.push(i));
	tokenize_related::<Attributes>(world, &mut items, entity, tokenize_attribute_tokens)?;
	tokenize_related::<Children>(world, &mut items, entity, tokenize_node_tree)?;

	items.xmap(maybe_tuple).xok()
}

fn tokenize_block_node_exprs(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) = world.entity(entity).get::<ItemOf::<BlockNode,SendWrapper<Expr>>>() {
		let block_node = expr.into_custom_token_stream();
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
		items.push(attr_expr.into_custom_token_stream());
	}
	if let Some(attr_key) = entity.get::<AttributeKeyExpr>() {
		items.push(attr_key.into_custom_token_stream());
	}
	if let Some(attr_val) = entity.get::<AttributeValueExpr>() {
		items.push(attr_val.into_custom_token_stream());
	}
	// tokenize_related::<Children>(world, &mut items, entity.id(), tokenize_node_tree)?;
let attr_entity = entity.id();
	if let Some(attr) =
				tokenize_combinator_exprs_to_node_tree(world, attr_entity)?
	{
				items.push(attr);
	}

	maybe_tuple(items).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;
	use beet_utils::prelude::*;

	fn parse_rstml(tokens: TokenStream) -> Matcher<String> {
		rstml_to_token_tree(tokens, WorkspacePathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	fn parse_combinator(tokens: &str) -> Matcher<String> {
		tokenize_combinator_tree(tokens, WorkspacePathBuf::new(file!()))
			.unwrap()
			.to_string().replace(" ", "")
			.xpect()
	}

	#[test]
	fn tag_only() {
		parse_rstml(quote! {<br/>}).to_be(
			quote! {
				related ! {
					Children [
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						)
					]
				}
			}
			.to_string(),
		);
		parse_rstml(quote! {<Foo/>}).to_be(
			quote! {
				related ! {
					Children [
						(
							NodeTag(String::from("Foo")),
							FragmentNode,
							TemplateNode
						)
					]
				}
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
				related ! {
					Children [
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true },
							related ! {
								Attributes [
									AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("hidden"))),
									(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("class"))), 			AttributeValueExpr(SendWrapper::new(syn::parse_quote!("foo")))),
									(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("some_key"))) , 	AttributeValueExpr(SendWrapper::new(syn::parse_quote!({ bar })))),
									(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("onmousemove"))), AttributeValueExpr(SendWrapper::new(syn::parse_quote!("some_js_func")))),
									(AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("onclick"))), 		AttributeValueExpr(SendWrapper::new(syn::parse_quote!({ |_: Trigger<OnClick>| {} }))))
								]
							}
						)
					]
				}
			}
			.to_string(),
		);
	}
	#[test]
	fn block_node() {
		parse_rstml(quote! {<div>{7}</div>}).to_be(
			quote! {
				related! {
					Children [
						(
							NodeTag(String::from("div")),
							ElementNode { self_closing: false },
							related! {
								Children [
									(
										BlockNode,
										ItemOf::<beet_common::node::rsx_nodes::BlockNode, send_wrapper::SendWrapper<syn::expr::Expr> > {
											value: SendWrapper::new(syn::parse_quote!({ 7 })),
											phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::BlockNode>
										}
									)
								]
							}
						)
					]
				}
			}
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
				related ! { 
					Children [{ 
						(
							NodeTag (String :: from ("br")), 
							ElementNode { self_closing : true }, 
							related ! { 
								Attributes [
									AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("hidden"))), 
									(AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("class"))), AttributeValueExpr (SendWrapper::new(syn::parse_quote!("foo")))), 
									(AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("onmousemove"))), AttributeValueExpr (SendWrapper::new(syn::parse_quote!("some_js_func")))), 
									(AttributeKeyExpr (SendWrapper::new(syn::parse_quote!("onclick"))), {|_:Trigger<OnClick>| { } })
								]
							}
						)
					}]
				}
			}
			.to_string().replace(" ", ""),
		);
	}
	#[test]
	fn nested_combinator() {
		parse_combinator(r#"
			<br 
				foo={
					let class = "bar";
					<div class={class}/>
				}
			/>
		"#).to_be(
			quote! {
				related ! {
					Children [{
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true },
							related ! {
								Attributes [(
									AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("foo"))),
									{
										let class = "bar";
										(
											NodeTag(String::from("div")),
											ElementNode { self_closing: true },
											related ! {
												Attributes [(
													AttributeKeyExpr(SendWrapper::new(syn::parse_quote!("class"))),
													{ class }
												)]
											}
										)
									}
								)]
							}
						)
					}]
				}
			}
			.to_string().replace(" ", ""),
		);
	}


}

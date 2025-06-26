use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use beet_utils::prelude::*;


/// Create a [`TokenStream`] of a [`Bundle`] that represents the *tokenized*
/// tree of nodes for the given [`Entity`], as opposed to the *finalized* tree,
#[rustfmt::skip]
pub fn tokenize_bundle_tokens(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let mut items = Vec::new();
	tokenize_idxs(world, &mut items, entity)?;
	tokenize_rsx_nodes(world, &mut items, entity)?;
	tokenize_rsx_directives(world, &mut items, entity)?;
	tokenize_web_nodes(world, &mut items, entity)?;
	tokenize_web_directives(world, &mut items, entity)?;
	tokenize_node_exprs_tokens(world, &mut items, entity)?;
	tokenize_combinator_exprs_tokens(world,&mut items, entity)?;
	tokenize_related::<Attributes>(world, &mut items, entity, tokenize_attribute_tokens)?;
	tokenize_related::<Children>(world, &mut items, entity, tokenize_bundle_tokens)?;
	
	items.xmap(unbounded_bundle).xok()
}

fn tokenize_node_exprs_tokens(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) = world.entity(entity).get::<NodeExpr>() {
		items.push(expr.self_token_stream());
	}
	Ok(())
}
fn tokenize_combinator_exprs_tokens(
	world: &World,
		items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) = tokenize_combinator_exprs_mapped(world, entity, tokenize_bundle_tokens)?{
		items.push(expr.self_token_stream());
	}
	Ok(())
}


fn tokenize_attribute_tokens(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let entity = world.entity(entity);
	let mut items = Vec::new();
	if let Some(attr_key) = entity.get::<AttributeKey>() {
		items.push(attr_key.self_token_stream());
	}
	if let Some(attr_val) = entity.get::<AttributeLit>() {
		items.push(attr_val.self_token_stream());
	}
	if let Some(attr_expr) = entity.get::<NodeExpr>() {
		items.push(attr_expr.self_token_stream());
	}
	let attr_entity = entity.id();
	// we dont care if its an attr block, just self tokenizing
	tokenize_combinator_exprs_tokens(world,&mut items, attr_entity)?;

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
				// .skip(33)
				.collect::<String>()
				.chars()
				.rev()
				// .skip(4)
				.collect::<String>()
				.chars()
				.rev()
				.collect::<String>()
				.xpect()
	}

	#[test]
	fn tag_only() {
		parse_rstml(quote! {<br/>}).to_be_str(
			quote! {
				(
					MacroIdx { 
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"), 
						start: LineCol { line: 1u32, col: 0u32 }},
					NodeTag(String::from("br")),
					ElementNode { self_closing: true }
				)
			}
			.to_string(),
		);
		parse_rstml(quote! {<Foo/>}).to_be_str(
			quote! {
				(
					MacroIdx {
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
						start: LineCol { line: 1u32, col: 0u32 }
					},
					ExprIdx(0u32),
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
				party_time=true
				some_key={bar}
				onmousemove="some_js_func"
				onclick={|_: Trigger<OnClick>| {}}
			/>}).to_be_str(
			quote! {
				(
					MacroIdx {
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
						start: LineCol { line: 1u32, col: 0u32 }
					},
					NodeTag(String::from("br")),
					ElementNode { self_closing: true },
					related! { Attributes[
						AttributeKey(String::from("hidden")),
						(
							AttributeKey(String::from("class")),
							AttributeLit::String(String::from("foo")),
							NodeExpr(SendWrapper::new(syn::parse_quote!("foo")))
						),
						(
							AttributeKey(String::from("party_time")),
							AttributeLit::Boolean(true),
							NodeExpr(SendWrapper::new(syn::parse_quote!(true)))
						),
						(
							AttributeKey(String::from("some_key")),
							NodeExpr(SendWrapper::new(syn::parse_quote!({ bar })))
						),
						(
							AttributeKey(String::from("onmousemove")),
							AttributeLit::String(String::from("some_js_func")),
							NodeExpr(SendWrapper::new(syn::parse_quote!("some_js_func")))
						),
						(
							AttributeKey(String::from("onclick")),
							NodeExpr(SendWrapper::new(syn::parse_quote!({ |_: Trigger<OnClick>| {} })))
						)
					]}
				)
			}
			.to_string(),
		);
	}
	#[test]
	fn block_node() {
		parse_rstml(quote! {<div>{7}</div>}).to_be_str(
			quote! {(
				MacroIdx {
					file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
					start: LineCol { line: 1u32, col: 0u32 }
				},
				NodeTag(String::from("div")),
				ElementNode { self_closing: false },
				related!{Children[(
					ExprIdx(0u32),
					BlockNode,
					NodeExpr(SendWrapper::new(syn::parse_quote!({ 7 })))
				)]}
				)
			}
			.to_string(),
		);
	}
	#[test]
	fn combinator_simple() {
		parse_combinator("<br/>").to_be_str(
			quote! {
				(
					MacroIdx {
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
						start: LineCol { line: 1u32, col: 0u32 }
					},
					NodeExpr(SendWrapper::new(syn::parse_quote!({
						(
							FragmentNode,
							related! {
								Children[
									(
										NodeTag(String::from("br")),
										ElementNode { self_closing: true }
									)
								]
							}
						)
					})))
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
				.xpect().to_be_str(
			quote! {
				(
					MacroIdx {
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
						start: LineCol { line: 1u32, col: 0u32 }
					},
					NodeExpr(SendWrapper::new(syn::parse_quote!({
						(
							FragmentNode,
							related! {
								Children[
									(
										NodeTag(String::from("br")),
										ElementNode { self_closing: true }
									),
									(
										NodeTag(String::from("br")),
										ElementNode { self_closing: true }
									)
								]
							}
						)
					})))
				)
			}
			.to_string(),
		);
	}

	#[test]
	fn combinator() {
		parse_combinator(r#"
			<br 
				hidden
				class=true
				onmousemove="some_js_func"
				onclick={|_: Trigger<OnClick>| {}}
			/>
		"#).to_be_str(
			quote! {
					(
						MacroIdx {
							file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
							start: LineCol { line: 1u32, col: 0u32 }
						},
						NodeExpr(SendWrapper::new(syn::parse_quote!({
							(
								FragmentNode,
								related! {
									Children[
										(
											NodeTag(String::from("br")),
											ElementNode { self_closing: true },
											related! {
												Attributes[
													AttributeKey(String::from("hidden")),
													(
														AttributeKey(String::from("class")),
														AttributeLit::Boolean(true),
														NodeExpr(SendWrapper::new(syn::parse_quote!(true)))
													),
													(
														AttributeKey(String::from("onmousemove")),
														AttributeLit::String(String::from("some_js_func")),
														NodeExpr(SendWrapper::new(syn::parse_quote!("some_js_func")))
													),
													(
														AttributeKey(String::from("onclick")),
														NodeExpr(SendWrapper::new(syn::parse_quote!({|_: Trigger<OnClick>| {}})))
													)
												]
											}
										)
									]
								}
							)
						})))
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
			/>"#).to_be_str(
			quote! {
						(
							MacroIdx {
								file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle_tokens.rs"),
								start: LineCol { line: 1u32, col: 0u32 }
							},
							NodeExpr(SendWrapper::new(syn::parse_quote!({
								(
									FragmentNode,
									related! {
										Children[
											(
												NodeTag(String::from("br")),
												ElementNode { self_closing: true },
												related! {
													Attributes[
														(
															AttributeKey(String::from("foo")),
															NodeExpr(SendWrapper::new(syn::parse_quote!({
																let class = "bar";
																(
																	NodeTag(String::from("div")),
																	ElementNode { self_closing: true },
																	related! {
																		Attributes[
																			(
																				AttributeKey(String::from("class")),
																				NodeExpr(SendWrapper::new(syn::parse_quote!({ class })))
																			)
																		]
																	}
																)
															})))
														)
													]
												}
											)
										]
									}
								)
							})))
						)
					}
			.to_string().replace(" ", "")
		);
	}


}

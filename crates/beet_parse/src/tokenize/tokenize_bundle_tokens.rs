use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;


/// Create a [`TokenStream`] of a [`Bundle`] that represents the *tokenized*
/// tree of nodes for the given [`Entity`], as opposed to the *finalized* tree,
#[rustfmt::skip]
pub fn tokenize_bundle_tokens(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let mut items = Vec::new();
	RsxComponents::tokenize_if_present(&world, &mut items, entity);
	tokenize_node_exprs_tokens(world, &mut items, entity)?;
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

fn tokenize_attribute_tokens(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let entity = world.entity(entity);
	let mut items = Vec::new();
	if let Some(attr_key) = entity.get::<AttributeKey>() {
		items.push(attr_key.self_token_stream());
	}
	if let Some(attr_val) = entity.get::<TextNode>() {
		items.push(attr_val.self_token_stream());
	}
	if let Some(attr_expr) = entity.get::<NodeExpr>() {
		items.push(attr_expr.self_token_stream());
	}
	unbounded_bundle(items).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse_rstml(tokens: TokenStream) -> TokenStream {
		tokenize_rstml_tokens(tokens, WsPathBuf::new(file!())).unwrap()
	}

	fn parse_combinator(tokens: &str) -> TokenStream {
		tokenize_combinator_tokens(tokens, WsPathBuf::new(file!())).unwrap()
	}

	#[test]
	fn element_tag_only() { parse_rstml(quote! {<br/>}).xpect_snapshot(); }
	#[test]
	fn template_tag_only() { parse_rstml(quote! {<Foo/>}).xpect_snapshot(); }
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
		/>})
		.xpect_snapshot();
	}
	#[test]
	fn block_node() { parse_rstml(quote! {<div>{7}</div>}).xpect_snapshot(); }
	#[test]
	fn combinator_simple() { parse_combinator("<br/>").xpect_snapshot(); }
	#[test]
	fn combinator_siblings() {
		tokenize_combinator_tokens("<br/><br/>", WsPathBuf::new(file!()))
			.unwrap()
			.xpect_snapshot();
	}

	#[test]
	fn combinator() {
		parse_combinator(
			r#"
			<br
				hidden
				class=true
				onmousemove="some_js_func"
				onclick={|_: Trigger<OnClick>| {}}
			/>
		"#,
		)
		.xpect_snapshot();
	}
	#[test]
	fn nested_combinator() {
		parse_combinator(
			r#"<br
				foo={
					let class = "bar";
					<div class={class}/>
				}
			/>"#,
		)
		.xpect_snapshot();
	}
}

use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;




/// Tokenize a bsx tree from the given entity as root. The root is the [`SnippetRoot`]
/// with all children nested under, in bsx we flatten in the case of a single child.
pub fn tokenize_bsx_root(world: &World, entity: Entity) -> Result<TokenStream> {
	let children = world.entity(entity).get::<Children>();

	let inner = match children.map_or(0, |c| c.len()) {
		// no children, return unit
		0 => quote!(()),
		// if one child we unwrap the first child
		1 => tokenize_bsx(world, children.unwrap()[0])?,
		// otherwise we parse as normal, the parent will simply be a list of children
		_ => tokenize_bsx(world, entity)?,
	};
	quote! {
		ResolveSnippets::resolve(#inner)
	}
	.xok()
}
/// Recursively tokenize bsx for this entity
fn tokenize_bsx(world: &World, entity: Entity) -> Result<TokenStream> {
	let mut items = Vec::new();
	RsxComponents::tokenize_if_present(&world, &mut items, entity);
	// BsxComponents::tokenize_if_present(&world, &mut items, entity);
	if world.entity(entity).contains::<ElementNode>() {
		TokenizeTemplate { wrap_inner: false }
			.tokenize(world, &mut items, entity)?;
	}
	if world.entity(entity).contains::<TemplateNode>() {
		tokenize_struct(world, &mut items, entity)?;
	}
	tokenize_node_exprs(world, &mut items, entity)?;
	tokenize_related::<Children>(world, &mut items, entity, tokenize_bsx)?;
	items.xmap(unbounded_bundle).xok()
}

/// tokenize node expressions like the `{foobar}` in `<div>{foobar}</div>`
fn tokenize_node_exprs(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(block) = world.entity(entity).get::<NodeExpr>() {
		items.push(block.insert_deferred());
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;

	fn parse(tokens: TokenStream) -> TokenStream {
		tokens
			.xmap(|t| ParseRsxTokens::rstml_to_bsx(t, WsPathBuf::new(file!())))
			.unwrap()
	}

	#[test]
	fn empty() { parse(quote! {}).xpect_snapshot(); }
	#[test]
	fn single() { parse(quote! {<my-func/>}).xpect_snapshot(); }
	#[test]
	fn multiple() {
		parse(quote! {
		<my_func/>
		<MyStruct/>
		})
		.xpect_snapshot();
	}
	#[test]
	fn args() { parse(quote! {<func foo=bar/>}).xpect_snapshot(); }
	#[test]
	fn children() { parse(quote! {<func>foo{bar}</func>}).xpect_snapshot(); }
}

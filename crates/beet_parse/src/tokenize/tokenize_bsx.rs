use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;




/// Tokenize a bsx tree from the given entity as root. The root is the [`SnippetRoot`]
/// with all children nested under, in bsx we flatten in the case of a single child.
pub fn tokenize_bsx_root(world: &World, entity: Entity) -> Result<TokenStream> {
	let children = world.entity(entity).get::<Children>();

	match children.map_or(0, |c| c.len()) {
		// no children, return unit
		0 => quote!(()).xok(),
		// if one child we unwrap the first child
		1 => tokenize_bsx(world, children.unwrap()[0]),
		// otherwise we parse as normal, the parent will simply be a list of children
		_ => tokenize_bsx(world, entity),
	}
}
/// Recursively tokenize bsx for this entity
fn tokenize_bsx(world: &World, entity: Entity) -> Result<TokenStream> {
	let mut items = Vec::new();
	// BsxComponents::tokenize_if_present(&world, &mut items, entity);
	tokenize_functions(world, &mut items, entity)?;
	tokenize_structs(world, &mut items, entity)?;
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
		items.push(block.inner_parsed().self_token_stream());
	}
	Ok(())
}

fn tokenize_functions(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	if !entity.contains::<ElementNode>() {
		Ok(())
	} else {
		let tag = entity
			.get::<NodeTag>()
			.ok_or_else(|| bevyhow!("ElementNode must have a NodeTag"))?;
		// convert <foo-bar> into foo_bar for rust conventions
		use heck::ToSnakeCase;
		let tag = tag.to_snake_case();
		let func_name: Ident = syn::parse_str(&tag)?;
		entity_components.push(quote! {#func_name()});
		Ok(())
	}
}
fn tokenize_structs(
	world: &World,
	_entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	if !entity.contains::<TemplateNode>() {
		Ok(())
	} else {
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> TokenStream {
		tokens
			.xmap(|t| ParseRsxTokens::rstml_to_bsx(t, WsPathBuf::new(file!())))
			.unwrap()
	}

	#[test]
	fn empty() { parse(quote! {}).xpect_snapshot(); }
	#[test]
	fn single() { parse(quote! {<my-struct/>}).xpect_snapshot(); }
	// #[test]
	// fn invalid() { parse(quote! {<d-iv/>}).xpect_snapshot(); }
}

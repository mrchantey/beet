use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;

/// Create a [`TokenStream`] of a [`Bundle`] that represents the *finalized*
/// tree of nodes for the given [`Entity`], as opposed to the *tokenized* tree,
/// see [`tokenize_bundle_tokens`].
#[rustfmt::skip]
pub fn tokenize_bundle(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let mut items = Vec::new();
	RsxComponents::tokenize_if_present(&world, &mut items, entity);
	tokenize_element_attributes(world,&mut items, entity)?;
	tokenize_template(world,&mut items, entity)?;
	tokenize_node_exprs(world,&mut items, entity)?;
	tokenize_related::<Children>(world,&mut items, entity, tokenize_bundle)?;
	items
		.xmap(unbounded_bundle)
		.xok()
}

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


/// Calls [`tokenize_bundle`] and appends any diagnostics tokens like rstml
/// compile errors. Prefer this method for macros, and [`tokenize_bundle`] for
/// codegen.
pub fn tokenize_bundle_with_errors(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	// TODO insert errors
	let mut tokens = tokenize_bundle(world, entity)?;
	if let Some(diagnostics) = world.entity(entity).get::<TokensDiagnostics>() {
		let diagnostics = TokensDiagnostics((*diagnostics).clone());
		tokens.extend(diagnostics.into_tokens());
	}

	Ok(tokens)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn works() {
		quote! {
			<span hidden=true>
				<MyComponent foo="bar" client:load />
				<div/>
			</span>
		}
		.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
		.unwrap()
		.xpect()
		.to_be_snapshot();
	}

	#[test]
	fn multiple_root_children() {
		quote! {
			<br/>
			<br/>
		}
		.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
		.unwrap()
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn blocks() {
		quote! {{foo}}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
	#[test]
	fn attribute_blocks() {
		quote! {<input hidden=val/>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
	#[test]
	fn inner_text_empty() {
		quote! {<style></style>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
	#[test]
	fn inner_text() {
		quote! {<style is:inline>foo{}</style>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
	#[test]
	fn inner_text_src() {
		quote! {<code src="foo.rs"/>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
}

use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;


/// Calls [`tokenize_rsx`] then wraps in [`ResolveSnippets::resolve`]
pub fn tokenize_rsx_resolve_snippet(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	let bundle = tokenize_rsx(world, entity)?;
	quote! {
		ResolveSnippets::resolve(#bundle)
	}
	.xok()
}


/// Recursive function that creates a [`TokenStream`] of a [`Bundle`] that represents the *finalized*
/// tree of nodes for the given [`Entity`], as opposed to the *tokenized* tree,
/// see [`tokenize_rsx_tokens`].
pub fn tokenize_rsx(world: &World, entity: Entity) -> Result<TokenStream> {
	let mut items = Vec::new();
	RsxComponents::tokenize_if_present(&world, &mut items, entity);
	tokenize_element_attributes(world, &mut items, entity)?;
	tokenize_template(world, &mut items, entity)?;
	tokenize_node_exprs(world, &mut items, entity)?;
	tokenize_related::<Children>(world, &mut items, entity, tokenize_rsx)?;
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


/// Calls [`tokenize_rsx`] and appends any diagnostics tokens like rstml
/// compile errors. Prefer this method for macros, and [`tokenize_rsx`] for
/// codegen.
pub fn tokenize_rsx_with_errors(
	world: &World,
	entity: Entity,
) -> Result<TokenStream> {
	// TODO insert errors
	let mut tokens = tokenize_rsx(world, entity)?;
	if let Some(diagnostics) = world.entity(entity).get::<TokensDiagnostics>() {
		let diagnostics = TokensDiagnostics((*diagnostics).clone());
		tokens.extend(diagnostics.into_tokens());
	}

	Ok(tokens)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
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
		.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
		.unwrap()
		.xpect_snapshot();
	}

	#[test]
	fn multiple_root_children() {
		quote! {
			<br/>
			<br/>
		}
		.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
		.unwrap()
		.xpect_snapshot();
	}
	#[test]
	fn blocks() {
		quote! {{foo}}
			.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect_snapshot();
	}
	#[test]
	fn attribute_blocks() {
		quote! {<input hidden=val/>}
			.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect_snapshot();
	}
	#[test]
	fn inner_text_empty() {
		quote! {<style></style>}
			.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect_snapshot();
	}
	#[test]
	fn inner_text() {
		quote! {<style node:inline>foo{}</style>}
			.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect_snapshot();
	}
	#[test]
	fn inner_text_src() {
		quote! {<style src="foo.rs"/>}
			.xmap(|t| ParseRsxTokens::rstml_to_rsx(t, WsPathBuf::new(file!())))
			.unwrap()
			.xpect_snapshot();
	}
}

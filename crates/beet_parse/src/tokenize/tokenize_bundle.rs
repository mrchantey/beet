use crate::prelude::*;
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
	tokenize_idxs(world, &mut items, entity)?;
	tokenize_rsx_nodes(world,&mut items, entity)?;
	tokenize_rsx_directives(world,&mut items, entity)?;
	tokenize_web_nodes(world,&mut items, entity)?;
	tokenize_web_directives(world,&mut items, entity)?;
	tokenize_element_attributes(world,&mut items, entity)?;
	tokenize_template(world,&mut items, entity)?;
	tokenize_node_exprs(world,&mut items, entity)?;
	tokenize_combinator_exprs(world,&mut items, entity)?;
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
		items.push(block.node_bundle_tokens());
	}
	Ok(())
}
fn tokenize_combinator_exprs(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	if let Some(expr) = tokenize_combinator_exprs_mapped(world, entity, super::tokenize_bundle)?{
		items.push(expr.node_bundle_tokens());
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
	use bevy::prelude::*;
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
		.to_string()
		.xpect()
		.to_be_str(
			quote! {(
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle.rs"),start:LineCol{line:1u32,col:0u32}},
				NodeTag(String::from("span")),
				ElementNode { self_closing: false },
				related!(Attributes[(
					AttributeKey::new("hidden"),
					OnSpawnTemplate::new_insert(true.into_attribute_bundle())
				)]),
				related!{Children[(
						ExprIdx(0u32),
						NodeTag(String::from("MyComponent")),
						FragmentNode,
						TemplateNode,
						ClientLoadDirective,
						OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
							let template = <MyComponent as Props>::Builder::default().foo("bar").build();
							(
								#[cfg(not(target_arch = "wasm32"))]
								{ TemplateSerde::new(&template) },
								#[cfg(target_arch = "wasm32")]
								{ () },
								TemplateRoot::spawn(Spawn(template.into_node_bundle()))
							)
						}.into_node_bundle())
					), (
						NodeTag(String::from("div")),
						ElementNode { self_closing: true }
					)]}
				
			)}
			.to_string(),
		);
	}

	#[test]
	fn multiple_root_children() {
		quote! {
			<br/>
			<br/>
		}
		.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
		.unwrap()
		.to_string()
		.xpect()
		.to_be_str(
			quote! {
				(
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle.rs"),start:LineCol{line:1u32,col:0u32}},
					FragmentNode,
					related!{Children[
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						),
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						)
					]}
				)
			}
			.to_string(),
		);
	}
	#[test]
	fn blocks() {
		quote! {{foo}}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.to_string()
			.xpect()
			.to_be_str(
				quote! {(
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle.rs"),start:LineCol{line:1u32,col:0u32}},
					ExprIdx(0u32),
					BlockNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{foo}.into_node_bundle())
				)}
				.to_string(),
			);
	}
	#[test]
	fn attribute_blocks() {
		quote! {<input hidden=val/>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.to_string()
			.xpect()
			.to_be_str(
				quote! {(
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle.rs"),start:LineCol{line:1u32,col:0u32}},
					NodeTag(String::from("input")),
					ElementNode { self_closing: true },
					related!(Attributes [
						(
							AttributeKey::new("hidden"),
							OnSpawnTemplate::new_insert(val.into_attribute_bundle()),
							ExprIdx(0u32)
						)
					])
				)}
				.to_string(),
			);
	}
	#[test]
	fn lang_content() {
		quote! {<style></style>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.to_string()
			.xpect()
			.to_be_str(
				quote! {(
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle.rs"),start:LineCol{line:1u32,col:0u32}},
					NodeTag(String::from("style")),
					ElementNode { self_closing: false }
				)}
				.to_string(),
			);
		quote! {<style>foo</style>}
			.xmap(|t| tokenize_rstml(t, WsPathBuf::new(file!())))
			.unwrap()
			.to_string()
			.xpect()
			.to_be_str(
				quote! {(
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_bundle.rs"),start:LineCol{line:1u32,col:0u32}},
					NodeTag(String::from("style")),
					ElementNode { self_closing: false },
					LangContent::InnerText(String::from("foo"))
				)}
				.to_string(),
			);
	}
}

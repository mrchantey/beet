use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;

/// Create a [`TokenStream`] of a [`Bundle`] that represents the *finalized*
/// tree of nodes for the given [`Entity`], as opposed to the *tokenized* tree,
/// see [`tokenize_bundle_tokens`].
pub fn tokenize_bundle(
	world: &mut World,
	entity: Entity,
) -> Result<TokenStream> {
	// The root is not an actual node, so we flatten the children if its 1, or
	// convert to a fragment.
	flatten_fragment(world, entity, tokenize_bundle_no_flatten)
}


/// Calls [`tokenize_bundle`] and appends any diagnostics tokens like rstml
/// compile errors. Prefer this method for macros, and [`tokenize_bundle`] for
/// codegen.
pub fn tokenize_bundle_with_errors(
	world: &mut World,
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



#[rustfmt::skip]
pub(super) fn tokenize_bundle_no_flatten(world: &World, entity: Entity) -> Result<TokenStream> {
	let mut items = Vec::new();
	tokenize_rsx_nodes(world,&mut items, entity)?;
	tokenize_rsx_directives(world,&mut items, entity)?;
	tokenize_web_nodes(world,&mut items, entity)?;
	tokenize_web_directives(world,&mut items, entity)?;
	tokenize_element_attributes(world,&mut items, entity)?;
	tokenize_template_attributes(world,&mut items, entity)?;
	tokenize_block_node_exprs(world, entity)?.map(|i|items.push(i));
	tokenize_combinator_exprs(world, entity)?.map(|i|items.push(i));
	tokenize_related::<Children>(world,&mut items, entity, tokenize_bundle_no_flatten)?;
	items
		.xmap(maybe_tuple)
		.xok()
}

/// the rstml macro parses in steps, ie <div foo={rsx!{<bar/>}}/> will resolve
/// the `bar` node first.
/// the combinator, however, represents attribute value expressions as child nodes
/// ie `<div foo={<bar/>}/>` so we need to resolve the attribute values
/// before walking the node tree.
// pub(super) fn resolve_attribute_values(
// 	_: TempNonSendMarker,
// 	mut commands: Commands,
// 	builder: TokenizeBundle,
// 	attribute_values: Populated<Entity, (With<AttributeOf>, With<Children>)>,
// ) -> Result {
// for entity in attribute_values.iter() {
// 	let tokens = builder.tokenize_bundle(entity)?;
// 	// if parse2 becomes problematic use Expr::Verbatim(tokens)
// 	let expr = syn::parse2::<Expr>(tokens)?;
// 	commands
// 		.entity(entity)
// 		.insert(AttributeValueExpr::new(expr));
// }

// 	Ok(())
// }

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
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: false },
				related!(Attributes[(
					"hidden".into_attr_key_bundle(),
					true.into_attr_val_bundle()
				)]),
				Children::spawn(bevy::ecs::spawn::SpawnIter([(
						NodeTag(String::from("MyComponent")),
						FragmentNode,
						TemplateNode,
						ClientLoadDirective,
						ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::node::rusty_tracker::RustyTracker> {
							value: RustyTracker { index: 0u32, tokens_hash: 6523630531850795118u64 },
							phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
						},
						{
							let template = <MyComponent as Props>::Builder::default().foo("bar").build();
							#[allow(unused_braces)]
							(
								#[cfg(not(target_arch = "wasm32"))]
								{ TemplateSerde::new(&template) },
								#[cfg(target_arch = "wasm32")]
								{ () },
								TemplateRoot::spawn(Spawn(template.into_node_bundle()))
							)
						}
					), (
						NodeTag(String::from("div")),
						ElementNode { self_closing: true }
					)].into_iter()))
				
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
		.to_be(
			quote! {
				(
					FragmentNode,
					children![
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						),
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						)
					]
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
			.to_be(
				quote! {(
					BlockNode,
					{foo}.into_node_bundle()
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
			.to_be(
				quote! {(
					NodeTag(String::from("input")),
					ElementNode { self_closing: true },
					related!(Attributes [
						(
							"hidden".into_attr_key_bundle(),
							val.into_attr_val_bundle()
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
			.to_be(
				quote! {(
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
			.to_be(
				quote! {(
					NodeTag(String::from("style")),
					ElementNode { self_closing: false },
					LangContent::InnerText(String::from("foo")),
					Children::spawn(bevy::ecs::spawn::SpawnIter([
						TextNode(String::from("foo"))
						].into_iter()))
				)}
				.to_string(),
			);
	}
}

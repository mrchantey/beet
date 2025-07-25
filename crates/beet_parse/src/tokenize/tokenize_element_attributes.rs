use crate::prelude::NodeExpr;
use crate::tokenize::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::LitStr;


pub fn tokenize_element_attributes(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	if !entity.contains::<ElementNode>() {
		Ok(())
	} else if let Some(attrs) = entity.get::<Attributes>() {
		let mut attr_entities = Vec::new();
		for attr_entity in attrs.iter() {
			let key = maybe_spanned_attr_key(world, attr_entity).map(
				|(key_str, span)| {
					let key_lit = LitStr::new(&key_str, span);
					(key_str, key_lit)
				},
			);

			let value = world.entity(attr_entity).get::<NodeExpr>().cloned();

			let mut attr_components = Vec::new();
			match (key, value) {
				// 1. Key with value
				(Some((key_str, key)), Some(mut value)) => {
					if is_event(&key_str, &value) {
						// event syntax sugar (inferred trigger types)
						tokenize_event_handler(
							&key_str,
							key.span(),
							&mut value,
						)?;
					}
					attr_components.push(quote! {AttributeKey::new(#key)});
					attr_components.push(value.attribute_bundle_tokens());
				}
				// 3. Key without value
				(Some((_, key)), None) => {
					attr_components.push(quote! {AttributeKey::new(#key)});
				}
				// 4. Value without key (block/spread attribute)
				(None, Some(value)) => {
					entity_components.push(value.node_bundle_tokens());
				}
				// 5. No key or value, should be unreachable but no big deal
				(None, None) => {}
			}
			if let Some(expr_idx) = world.entity(attr_entity).get::<ExprIdx>() {
				attr_components.push(expr_idx.self_token_stream());
			}

			if attr_components.len() == 1 {
				attr_entities.push(attr_components.pop().unwrap());
			} else if !attr_components.is_empty() {
				attr_entities.push(quote! {
					(#(#attr_components),*)
				});
			}
		}
		if !attr_entities.is_empty() {
			entity_components.push(quote! {
						related!(Attributes[
						#(#attr_entities),*
					])
			});
		}
		Ok(())
	} else {
		Ok(())
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> Matcher<String> {
		tokenize_rstml(tokens, WsPathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	#[test]
	fn key_value() {
		quote! {
			<span hidden/>
		}
		.xmap(parse)
		.to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_element_attributes.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[(
					NodeTag(String::from("span")),
					ElementNode { self_closing: true },
					related!(Attributes[
						AttributeKey::new("hidden")
					])
				)]}
			)}
			.to_string(),
		);
		quote! {
			<span hidden=true/>
		}
		.xmap(parse)
		.to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_element_attributes.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[(
					NodeTag(String::from("span")),
					ElementNode { self_closing: true },
					related!(Attributes[(
						AttributeKey::new("hidden"),
						OnSpawnTemplate::new_insert(true.into_attribute_bundle())
					)])
				)]}
			)}
			.to_string(),
		);
	}
	#[test]
	fn block() {
		quote! {
			<span {foo}/>
		}
		.xmap(parse)
		.to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_element_attributes.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[(
					ExprIdx(0u32),
					NodeTag(String::from("span")),
					ElementNode { self_closing: true },
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{foo}.into_node_bundle())
				)]}
			)}
			.to_string(),
		);
	}
	#[test]
	fn events() {
		quote! {
			<span onclick={foo}/>
		}
		.xmap(parse)
		.to_be_str(
			quote! {
				(
					BeetRoot,
					InstanceRoot,
					MacroIdx {
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_element_attributes.rs"),
						start: LineCol { line: 1u32, col: 0u32 }
					},
					FragmentNode,
					related! {
						Children[(
							NodeTag(String::from("span")),
							ElementNode { self_closing: true },
							related!(Attributes[(
								AttributeKey::new("onclick"),
								OnSpawnTemplate::new_insert(#[allow(unused_braces)]{foo}.into_attribute_bundle()),
								ExprIdx(0u32)
							)])
						)]
					}
				)
			}
			.to_string(),
		);
		quote! {
			<span onclick="some_js_func"/>
		}
		.xmap(parse)
		.to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_element_attributes.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[(
					NodeTag(String::from("span")),
					ElementNode { self_closing: true },
					related!(Attributes[(
						AttributeKey::new("onclick"),
						OnSpawnTemplate::new_insert("some_js_func".into_attribute_bundle())
					)])
				)]}
			)}
			.to_string(),
		);
	}
	#[test]
	fn all() {
		quote! {
			<span
				hidden
				class="foo"
				{foo}
				onmousemove="some_js_func"
				onclick=|| { println!("clicked"); }
			/>
		}
		.xmap(parse)
		.to_be_str(
			quote! {
				(
					BeetRoot,
					InstanceRoot,
					MacroIdx {
						file: WsPathBuf::new("crates/beet_parse/src/tokenize/tokenize_element_attributes.rs"),
						start: LineCol { line: 1u32, col: 0u32 }
					},
					FragmentNode,
					related! {
						Children[(
							ExprIdx(0u32),
							NodeTag(String::from("span")),
							ElementNode { self_closing: true },
							OnSpawnTemplate::new_insert(#[allow(unused_braces)]{foo}.into_node_bundle()),
							related!(Attributes[
								AttributeKey::new("hidden"),
								(
									AttributeKey::new("class"),
									OnSpawnTemplate::new_insert("foo".into_attribute_bundle())
								),
								(
									AttributeKey::new("onmousemove"),
									OnSpawnTemplate::new_insert("some_js_func".into_attribute_bundle())
								),
								(
									AttributeKey::new("onclick"),
									OnSpawnTemplate::new_insert(
										#[allow(unused_braces)]{|_: Trigger<OnClick>| { println!("clicked"); }}.into_attribute_bundle()
									),
									ExprIdx(1u32)
								)
							])
						)]
					}
				)
			}
			.to_string(),
		);
	}
}

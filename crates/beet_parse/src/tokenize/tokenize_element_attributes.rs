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
					attr_components.push(quote! {AttributeKey::new(#key)});
					// both events and attributes get a key
					// attribute values added to child entity,
					// event handlers added to parent entity.
					// this technique is also used in `derive_attribute_block.rs`
					if is_event(&key_str, &value) {
						// event syntax sugar (inferred trigger types)
						tokenize_event_handler(
							&key_str,
							key.span(),
							&mut value,
						)?;
						let parsed = value.as_bundle();
						attr_components.push(quote! {
								OnSpawnDeferred::insert_parent::<AttributeOf>(#parsed)
						});
					} else {
						attr_components.push(value.insert_deferred());
					}
				}
				// 3. Key without value
				(Some((_, key)), None) => {
					attr_components.push(quote! {AttributeKey::new(#key)});
				}
				// 4. Value without key (block/spread attribute)
				(None, Some(value)) => {
					entity_components.push(value.insert_deferred());
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
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> Matcher<TokenStream> {
		tokenize_rstml(tokens, WsPathBuf::new(file!()))
			.unwrap()
			.xpect()
	}

	#[test]
	fn key() {
		quote! {
			<span hidden/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
	#[test]
	fn key_value() {
		quote! {
			<span hidden=true/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
	#[test]
	fn block() {
		quote! {
			<span {foo}/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
	#[test]
	fn events() {
		quote! {
			<span onclick={foo}/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
	#[test]
	fn js_events() {
		quote! {
			<span onclick="some_js_func"/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
	#[test]
	fn lang_src() {
		quote! {
			<style src="./foobar"/>
		}
		.xmap(parse)
		.to_be_snapshot();
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
		.to_be_snapshot();
	}
}

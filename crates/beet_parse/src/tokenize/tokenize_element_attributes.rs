use crate::tokenize::*;
use beet_common::prelude::*;
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

			let value = first_attribute_expr(world, attr_entity)?;


			let mut attr_components = Vec::new();
			match (key, value) {
				// 1: Events
				(Some((key_str, key)), Some(value))
					if is_event(&key_str, &value) =>
				{
					let value =
						tokenize_event_handler(&key_str, key.span(), value)?;
					entity_components.push(quote! {EventKey::new(#key_str)});
					entity_components.push(quote! {#value.into_node_bundle()});
				}
				// 2. Key with value
				(Some((_, key)), Some(value)) => {
					attr_components.push(quote! {AttributeKey::new(#key)});
					attr_components
						.push(quote! {#value.into_attr_val_bundle()});
				}
				// 3. Key without value
				(Some((_, key)), None) => {
					attr_components.push(quote! {AttributeKey::new(#key)});
				}
				// 4. Value without key (block/spread attribute)
				(None, Some(value)) => {
					entity_components.push(quote! {#value.into_node_bundle()});
				}
				// 5. No key or value, should be unreachable but no big deal
				(None, None) => {}
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
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				related!(Attributes[
					AttributeKey::new("hidden")
				])
			)}
			.to_string(),
		);
		quote! {
			<span hidden=true/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				related!(Attributes[(
					AttributeKey::new("hidden"),
					true.into_attr_val_bundle()
				)])
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
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				{foo}.into_node_bundle()
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
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				EventKey::new("onclick"),
				{foo}.into_node_bundle()
			)}
			.to_string(),
		);
		quote! {
			<span onclick="some_js_func"/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				related!(Attributes[(
					AttributeKey::new("onclick"),
					"some_js_func".into_attr_val_bundle()
				)])
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
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				{foo}.into_node_bundle(),
				EventKey::new("onclick"),
				{|_: Trigger<OnClick>| { println!("clicked"); }}.into_node_bundle(),
				related!(Attributes[
					AttributeKey::new("hidden"),
					(
						AttributeKey::new("class"),
						"foo".into_attr_val_bundle()
					),
					(
						AttributeKey::new("onmousemove"),
						"some_js_func".into_attr_val_bundle()
					)
				])
			)}
			.to_string(),
		);
	}
}

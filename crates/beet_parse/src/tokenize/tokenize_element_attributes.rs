use crate::tokenize::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;




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
			if try_event_observer(world, entity_components, attr_entity)? {
				continue;
			}

			let mut attr_components = Vec::new();
			// blocks ie <span {Vec3::new()} />
			// inserted directly as an entity component
			if let Some(attr) =
				maybe_spanned_expr::<AttributeExpr>(world, attr_entity)?
			{
				entity_components.push(quote! {#attr.into_node_bundle()});
			}

			if let Some(attr) =
				maybe_spanned_expr::<AttributeKeyExpr>(world, attr_entity)?
			{
				attr_components.push(quote! {#attr.into_attr_key_bundle()});
			}
			if let Some(attr) =
				maybe_spanned_expr::<AttributeValueExpr>(world, attr_entity)?
			{
				attr_components.push(quote! {#attr.into_attr_val_bundle()});
			}
			if let Some(attr) = tokenize_combinator_exprs(world, attr_entity)? {
				if world.entity(attr_entity).contains::<AttributeKeyExpr>() {
					// if this attribute has a key, the combinator must be a value
					attr_components.push(quote! {#attr.into_attr_val_bundle()});
				} else {
					// otherwise the combinator is a block value, aka a component
					entity_components.push(attr);
				}
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
					"hidden".into_attr_key_bundle()
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
					"hidden".into_attr_key_bundle(),
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
				EntityObserver::new(#[allow(unused_braces)]{foo})
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
					"onclick".into_attr_key_bundle(),
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
				EntityObserver::new(#[allow(unused_braces)] |_: Trigger<OnClick>| { println!("clicked"); }),
				related!(Attributes[
					"hidden".into_attr_key_bundle(),
					(
						"class".into_attr_key_bundle(),
						"foo".into_attr_val_bundle()
					),
					(
						"onmousemove".into_attr_key_bundle(),
						"some_js_func".into_attr_val_bundle()
					)
				])
			)}
			.to_string(),
		);
	}
}

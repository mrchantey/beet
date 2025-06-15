use crate::tokenize::*;
use crate::utils::expr_to_ident;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Ident;

pub fn tokenize_template_attributes(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	if !entity.contains::<TemplateNode>() {
		return Ok(());
	}

	let Some(node_tag) = entity.get::<NodeTag>() else {
		return Ok(());
	};
	let node_tag_span = entity.get::<ItemOf<NodeTag, SendWrapper<Span>>>();
	let Some(tracker) = entity.get::<ItemOf<TemplateNode, RustyTracker>>()
	else {
		return Ok(());
	};
	let mut prop_assignments = Vec::new();

	if let Some(attrs) = entity.get::<Attributes>() {
		for attr_entity in attrs.iter() {
			if let Some(attr) =
				maybe_spanned_expr::<AttributeExpr>(world, attr_entity)?
			{
				entity_components.push(quote! {#attr.into_node_bundle()});
			}
			let combinator_attr =
				tokenize_combinator_exprs(world, attr_entity)?;

			if let Some(key) =
				maybe_spanned_expr::<AttributeKeyExpr>(world, attr_entity)?
				&& let Some(key) = expr_to_ident(&key)
			{
				if let Some(val) = combinator_attr {
					// first check if there was a combinator value
					prop_assignments.push(quote! {.#key(#val)});
				} else {
					// otherwise check if theres a regular value
					let value = maybe_spanned_expr::<AttributeValueExpr>(
						world,
						attr_entity,
					)?
					.unwrap_or_else(|| {
						// finally no value means a bool flag
						syn::parse_quote! {true}
					});
					prop_assignments.push(quote! {.#key(#value)});
				}
			} else if let Some(value) = combinator_attr {
				// if it doesnt have a key, the combinator must be a block value
				entity_components.push(value);
			}
		}
	}

	let template_ident = Ident::new(
		&node_tag.as_str(),
		node_tag_span.map(|s| ***s).unwrap_or(Span::call_site()),
	);

	// we create an inner tuple, so that we can define the template
	// and reuuse it for serialization
	let mut inner_items = Vec::new();
	if entity.contains::<ClientLoadDirective>()
		|| entity.contains::<ClientOnlyDirective>()
	{
		inner_items.push(quote! {
			#[cfg(not(target_arch = "wasm32"))]
			{TemplateSerde::new(&template)},
			#[cfg(target_arch = "wasm32")]
			{()}
		});
	}
	// the output of a template is *children!*, ie the template is a fragment.
	// this is important to avoid duplicate components like NodeTag
	inner_items
		.push(quote! {TemplateRoot::spawn(Spawn(template.into_node_bundle()))});
	entity_components.push(tracker.into_custom_token_stream());

	entity_components.push(quote! {{
		let template = <#template_ident as Props>::Builder::default()
				#(#prop_assignments)*
				.build();
		#[allow(unused_braces)]
		(#(#inner_items),*)
	}});
	Ok(())
}

use crate::prelude::NodeExpr;
use crate::tokenize::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn tokenize_template(
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
	let node_tag_span = entity.get::<SpanOf<NodeTag>>();
	let mut prop_assignments = Vec::new();

	if let Some(attrs) = entity.get::<Attributes>() {
		for attr_entity in attrs.iter() {
			let key = maybe_spanned_attr_key(world, attr_entity).map(
				|(key, span)| {
					let ident = Ident::new(&key, span);
					(key, ident)
				},
			);

			let value = world.entity(attr_entity).get::<NodeExpr>().cloned();

			match (key, value) {
				// 1: Events
				(Some((key_str, key)), Some(mut value))
					if is_event(&key_str, &value) =>
				{
					tokenize_event_handler(&key_str, key.span(), &mut value)?;
					let value = value.inner_parsed();
					prop_assignments.push(quote! {.#key(#value)});
				}
				// 2. Key with value
				(Some((_, key)), Some(value)) => {
					let value = value.inner_parsed();
					prop_assignments.push(quote! {.#key(#value)});
				}
				// 3. Key without value (boolean attribute)
				(Some((_, key)), None) => {
					prop_assignments.push(quote! {.#key(true)});
				}
				// 4. Value without key (block/spread attribute)
				(None, Some(value)) => {
					entity_components.push(value.node_bundle_tokens());
				}
				// 5. No key or value, should be unreachable but no big deal
				(None, None) => {}
			}
		}
	}

	let template_ident = Ident::new(
		&node_tag.as_str(),
		node_tag_span.map(|s| **s).unwrap_or(Span::call_site()),
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

	let items = unbounded_bundle(inner_items);

	let node_expr = NodeExpr::new_block(syn::parse_quote! {{
		let template = <#template_ident as Props>::Builder::default()
				#(#prop_assignments)*
				.build();
		#items
	}});

	entity_components.push(node_expr.node_bundle_tokens());
	Ok(())
}

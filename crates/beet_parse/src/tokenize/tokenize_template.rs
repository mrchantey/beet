use crate::prelude::NodeExpr;
use crate::tokenize::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;
use syn::Ident;


pub struct TokenizeTemplate {
	pub wrap_inner: bool,
}

impl TokenizeTemplate {
	pub fn tokenize(
		&self,
		world: &World,
		entity_components: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()> {
		let entity = world.entity(entity);
		let Some(node_tag) = entity.get::<NodeTag>() else {
			return Ok(());
		};
		let node_tag_span = entity.get::<SpanOf<NodeTag>>();
		let mut prop_assignments = Vec::new();

		if let Some(attrs) = entity.get::<Attributes>() {
			for attr_entity in attrs.iter() {
				let key = maybe_spanned_attr_key(world, attr_entity).map(
					|(key, span)| {
						let ident = non_reserved_key(&key, span);
						(key, ident)
					},
				);

				let value =
					world.entity(attr_entity).get::<NodeExpr>().cloned();

				match (key, value) {
					// 1: Events
					(Some((key_str, key)), Some(mut value))
						if is_event(&key_str, &value) =>
					{
						tokenize_event_handler(
							&key_str,
							key.span(),
							&mut value,
						)?;
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
						entity_components.push(value.insert_deferred());
					}
					// 5. No key or value, should be unreachable but no big deal
					(None, None) => {}
				}
			}
		}

		let template_ident = Ident::new(
			// normalize both <element-types> and <ElementTypes>
			&node_tag.as_str().to_upper_camel_case(),
			node_tag_span.map(|s| **s).unwrap_or(Span::call_site()),
		);

		let mut template_def: Expr = syn::parse_quote! {
				<#template_ident as Props>::Builder::default()
				#(#prop_assignments)*
				.build()
		};

		if self.wrap_inner {
			template_def = if entity.contains::<ClientLoadDirective>()
				|| entity.contains::<ClientOnlyDirective>()
			{
				// this also adds a TemplateRoot::spawn() via component hook using a reflect clone
				syn::parse_quote! {
					ClientIslandRoot::new(#template_def)
				}
			} else {
				syn::parse_quote! {
					TemplateRoot::spawn(Spawn(#template_def.into_bundle()))
				}
			};
		}
		entity_components.push(NodeExpr::new(template_def).insert_deferred());
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> TokenStream {
		ParseRsxTokens::rstml_to_rsx(tokens, WsPathBuf::new(file!())).unwrap()
	}

	#[test]
	fn key_value() {
		quote! {
			<Foo bar client:load/>
		}
		.xmap(parse)
		.xpect_snapshot();
	}
	#[test]
	fn reserved_names() {
		quote! {
			<Foo type="bar"/>
		}
		.xmap(parse)
		.xpect_snapshot();
	}
}

use crate::prelude::NodeExpr;
use crate::tokenize::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Expr;


pub fn tokenize_struct(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	let Some(node_tag) = entity.get::<NodeTag>() else {
		return Ok(());
	};
	let node_tag_span = entity.get::<SpanOf<NodeTag>>();
	let mut field_assignments = Vec::new();

	// if a 'no_default' attr is present, disable default
	let mut force_default = true;
	if let Some(attrs) = entity.get::<Attributes>() {
		for attr_entity in attrs.iter() {
			let key = maybe_spanned_attr_key(world, attr_entity).map(
				|(key, span)| {
					let ident = non_reserved_key(&key, span);
					(key, ident)
				},
			);

			let value = world.entity(attr_entity).get::<NodeExpr>().cloned();

			match (key, value) {
				// 2. Key with value
				(Some((_, key)), Some(value)) => {
					let value = value.inner_parsed();
					field_assignments.push(quote! {#key: #value});
				}
				// 3. Key without value (boolean attribute)
				(Some((key_str, key)), None) => {
					if key_str == "no_default" {
						force_default = false;
					} else {
						field_assignments.push(quote! {#key: true});
					}
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

	let template_ident: Expr = syn::parse_str(
		// normalize both <element-types> and <ElementTypes>
		&node_tag,
	)?;
	// apply the span
	let span = node_tag_span.map(|s| **s).unwrap_or(Span::call_site());
	let template_ident: Expr =
		syn::parse_quote_spanned! {span=>#template_ident};


	let no_attrs = field_assignments.is_empty();
	let is_constructor = node_tag.contains(":");

	let template_def = if no_attrs && is_constructor {
		// currently unsupported as rstml doesnt support constructors
		// https://github.com/rs-tml/rstml/issues/69
		// ie <Transform::new() />
		template_ident.to_token_stream()
	} else if no_attrs {
		// ie <Transform />
		quote!(#template_ident::default())
	} else if force_default {
		// ie <Transform position={..}/>
		quote!(#template_ident {
			#(#field_assignments),*,
			..default()
		})
	} else {
		// ie <Transform no_default/>
		quote!(#template_ident {
			#(#field_assignments),*,
		})
	};

	let inner = if entity.contains::<ClientLoadDirective>()
		|| entity.contains::<ClientOnlyDirective>()
	{
		// this also adds a TemplateRoot::spawn() via component hook using a reflect clone
		syn::parse_quote! {
			ClientIslandRoot::new(#template_def)
		}
	} else {
		syn::parse_quote! {
			#template_def
		}
	};
	entity_components.push(NodeExpr::new(inner).insert_deferred());

	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> TokenStream {
		ParseRsxTokens::rstml_to_bsx(tokens, WsPathBuf::new(file!())).unwrap()
	}

	#[test]
	fn empty() {
		quote! {
			<Foo/>
		}
		.xmap(parse)
		.xpect_snapshot();
	}

	#[test]
	fn key_value() {
		quote! {
			<Transform position=Vec3(0,0,0)/>
		}
		.xmap(parse)
		.xpect_snapshot();
	}

	#[test]
	fn no_default() {
		quote! {
			<Foo bar no_default/>
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

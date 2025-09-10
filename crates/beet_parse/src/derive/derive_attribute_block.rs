use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Result;


/// For a struct where each field implements `IntoBundle`
pub fn parse_derive_attribute_block(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = NodeField::parse_derive_input(&input)?;
	let idents = fields.iter().map(|f| &f.ident);

	let flattened = fields
		.iter()
		.filter(|field| field.field_attributes.contains("flatten"))
		.map(|field| &field.ident);
	let attrs = fields
		.iter()
		.filter(|field| !field.field_attributes.contains("flatten"))
		.map(|field| {
			let ident = &field.ident;
			let name_str = ident.to_string().replace('#', "");
			// attribute values added to child entity,
			// event handlers added to parent entity.
			// this technique is also used in `tokenize_element_attributes.rs`
			let (attr_bundle, event_bundle) = match name_str.starts_with("on") {
				true => (
					None,
					Some(quote! {
						world.entity_mut(parent_entity)
							.insert(#ident.into_bundle());
					}),
				),
				false => (Some(quote! {#ident.into_bundle(),}), None),
			};

			let inner = quote! {(
				AttributeOf::new(parent_entity),
				#attr_bundle
				AttributeKey::new(#name_str)
			)};
			if field.is_optional() {
				quote! {
					if let Some(#ident) = #ident {
						#event_bundle
						world.spawn(#inner);
					}
				}
			} else {
				quote! {
					#event_bundle
					world.spawn(#inner);
				}
			}
		});

	let target_name = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();


	// let attrs = unbounded_related(&format_ident!("Attributes"), attrs);
	// we cant use related! and friends here because duplicate components are not
	// allowed.
	let attrs = quote! {
		OnSpawn::new(move |entity| {
			let parent_entity = entity.id();
			entity.world_scope(move |world| {
				#(#attrs)*
			});
		})
	};

	let imports = dom_imports();

	Ok(quote! {
		#imports

		impl #impl_generics IntoBundle<Self> for #target_name #type_generics #where_clause {
		fn into_bundle(self) -> impl Bundle{
			let Self {
				#(#idents),*
			} = self;
			#[allow(unused_braces)]
			(
				#attrs,
				#(#flattened.into_bundle()),*
			)
			}
		}
	})
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		parse_derive_attribute_block(syn::parse_quote! {
			#[derive(Node)]
			#[node(into_rsx = my_node)]
			struct MyNode {
				present: u32,
				optional: Option<u32>,
				onclick: EventHandler<OnClick>,
				#[field(flatten)]
				nested: OtherBlock,
			}
		})
		.xpect_snapshot();
	}
}

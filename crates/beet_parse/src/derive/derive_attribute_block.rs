use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Result;


/// For a struct where each field implements `IntoTemplateBundle`
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
			// event handlers added to parent entity
			let (attr_bundle, event_bundle) = match name_str.starts_with("on") {
				true => (
					None,
					Some(quote! {
						world.entity_mut(parent_entity)
							.insert(#ident.into_node_bundle());
					}),
				),
				false => (Some(quote! {#ident.into_attribute_bundle(),}), None),
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
	Ok(quote! {
		use beet::prelude::*;

		impl #impl_generics IntoTemplateBundle<Self> for #target_name #type_generics #where_clause {
		fn into_node_bundle(self) -> impl Bundle{
			let Self {
				#(#idents),*
			} = self;
			#[allow(unused_braces)]
			(
				#attrs,
				#(#flattened.into_node_bundle()),*
			)
			}
		}
	})
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::quote;
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
		.to_string()
		.xpect()
		.to_be_str(
			quote! {
				use beet::prelude::*;
				impl IntoTemplateBundle<Self> for MyNode {
					fn into_node_bundle(self) -> impl Bundle {
						let Self { present, optional,onclick, nested } = self;
						#[allow(unused_braces)]
						(
							OnSpawn::new(move |entity| {
								let parent_entity = entity.id();
								entity.world_scope(move |world| {
									world.spawn((
										AttributeOf::new(parent_entity),
										present.into_attribute_bundle(),
										AttributeKey::new("present")
									));
									if let Some(optional) = optional {
										world.spawn((
											AttributeOf::new(parent_entity),
											optional.into_attribute_bundle(),
											AttributeKey::new("optional")
										));
									}
									world.entity_mut(parent_entity)
										.insert(onclick.into_node_bundle());
									world.spawn((
										AttributeOf::new(parent_entity),
										AttributeKey::new("onclick")
									));
								});
							}),
							nested.into_node_bundle()
						)
					}
				}
			}
			.to_string(),
		);
	}
}

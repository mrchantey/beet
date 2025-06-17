use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;



pub fn impl_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let input_ident = &input.ident;

	Ok(quote! {
		impl #impl_generics bevy::ecs::bundle::DynamicBundle for #input_ident #type_generics #where_clause {
			type Effect = Self;

			fn get_components(
				self,
				_func: &mut impl FnMut(
					bevy::ecs::component::StorageType,
					bevy::ptr::OwningPtr<'_>,
				),
			) -> Self::Effect {
				self
			}
		}

		unsafe impl #impl_generics bevy::ecs::bundle::Bundle for #input_ident #type_generics #where_clause {
			fn component_ids(
				_components: &mut bevy::ecs::component::ComponentsRegistrator,
				_ids: &mut impl FnMut(bevy::ecs::component::ComponentId),
			) {
			}

			fn get_component_ids(
				_components: &bevy::ecs::component::Components,
				_ids: &mut impl FnMut(Option<bevy::ecs::component::ComponentId>),
			) {
			}

			fn register_required_components(
				_components: &mut bevy::ecs::component::ComponentsRegistrator,
				_required_components: &mut bevy::ecs::component::RequiredComponents,
			) {
			}
		}
	})
}

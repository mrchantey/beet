use beet_utils::prelude::Xtend;
use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;



pub fn bundle_effect(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let effect_name = input
		.attrs
		.iter()
		.find_map(|attr| {
			if attr.path().is_ident("effect") {
				attr.parse_args::<TokenStream>().unwrap().xsome()
			} else {
				None
			}
		})
		.unwrap_or_else(|| quote! {Self::effect});


	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let input_ident = &input.ident;

	Ok(quote! {
		impl #impl_generics bevy::ecs::bundle::DynamicBundle for #input_ident #type_generics #where_clause {
			type Effect = Self;

	   unsafe fn get_components(ptr: bevy::ptr::MovingPtr<'_, Self>, _func: &mut impl FnMut(bevy::ecs::component::StorageType, bevy::ptr::OwningPtr<'_>)){
				 // Forget the pointer so that the value is available in `apply_effect`.
					std::mem::forget(ptr);
				}
		 unsafe fn apply_effect(mut ptr: bevy::ptr::MovingPtr<'_, std::mem::MaybeUninit<Self>>, entity: &mut EntityWorldMut){
				let effect = unsafe { ptr.assume_init() }.read();
				// let effect = unsafe { &mut *ptr.as_mut_ptr() };
				#effect_name(effect, entity);
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

		}
	})
}

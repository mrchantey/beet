use crate::utils::punctuated_args;
use crate::utils::CrateManifest;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

pub fn impl_action_attr(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let item = parse_macro_input!(item as DeriveInput);
	let result = parse(attr.into(), item);
	result.unwrap_or_else(|err| err.into_compile_error()).into()
}
fn parse(attr: TokenStream, mut item: DeriveInput) -> syn::Result<TokenStream> {
	assert_derive_component(&item)?;
	let (impl_generics, type_generics, where_clause) =
		item.generics.split_for_impl();

	let beet_flow_path = CrateManifest::get_path_direct("beet_flow");

	let ident_str = &item.ident.to_string();



	let on_add_ident = syn::Ident::new(
		&format!("OnAdd{}", ident_str),
		proc_macro2::Span::call_site(),
	);

	let turbofish = if item.generics.params.is_empty() {
		quote!()
	} else {
		quote!(::#type_generics)
	};

	item.attrs
	.push(syn::parse_quote!(#[component(on_add=#on_add_ident #turbofish, on_remove = #beet_flow_path::prelude::on_remove_action)]));
	item.attrs
		.push(syn::parse_quote!(#[require(#beet_flow_path::prelude::ActionObservers)]));

	let observers = punctuated_args(attr)?.into_iter().map(|observer| {
		quote! {cmd.observe(#observer);}
	});

	Ok(quote! {
		#[allow(non_snake_case)]
		fn #on_add_ident #impl_generics(
			mut world: bevy::ecs::world::DeferredWorld,
			action: bevy::ecs::entity::Entity,
			cid: bevy::ecs::component::ComponentId
		) #where_clause {
		  #beet_flow_path::prelude::ActionObservers::on_add(&mut world, action, cid,
			  |world, observer_entity| {
					let mut commands = world.commands();
				  let mut cmd = commands.entity(observer_entity);
					#(#observers)*
			  },
		  );
		}
		#item
	})
}

fn assert_derive_component(item: &DeriveInput) -> syn::Result<()> {
	if !item.attrs.iter().any(|attr| {
		attr.path().is_ident("derive")
			&& attr
				.meta
				.require_list()
				.ok()
				.map(|list| list.tokens.to_string().contains("Component"))
				.unwrap_or(false)
	}) {
		return Err(syn::Error::new_spanned(
			&item.ident,
			r#"
the #[action] macro must appear before #[derive(Component)]:
```rust
#[action(my_action)]
#[derive(Component)]
struct MyAction;
```
"#,
		));
	}
	Ok(())
}

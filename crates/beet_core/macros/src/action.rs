use beet_utils::prelude::AttributeGroup;
use beet_utils::prelude::pkg_ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

pub fn impl_action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let item = parse_macro_input!(item as DeriveInput);
	// let result = parse(attr.into(), item);
	let result = parse(attr.into(), item);
	result.unwrap_or_else(|err| err.into_compile_error()).into()
}

fn parse(attr: TokenStream, mut item: DeriveInput) -> syn::Result<TokenStream> {
	assert_derive_component(&item)?;
	let (impl_generics, type_generics, where_clause) =
		item.generics.split_for_impl();

	let ident_str = &item.ident.to_string();

	// ie OnAddMyComponent
	let on_add_ident = syn::Ident::new(
		&format!("OnAdd{}", ident_str),
		proc_macro2::Span::call_site(),
	);

	let turbofish_type_generics = if item.generics.params.is_empty() {
		quote!()
	} else {
		quote!(::#type_generics)
	};

	item.attrs
	.push(syn::parse_quote!(#[component(on_add=#on_add_ident #turbofish_type_generics)]));

	let observers = AttributeGroup::parse_punctated(attr)?;
	let beet_path = pkg_ext::internal_or_beet("beet_core");

	Ok(quote! {
		#[allow(non_snake_case)]
		fn #on_add_ident #impl_generics(
			mut world: #beet_path::prelude::DeferredWorld,
			cx: #beet_path::prelude::HookContext,
		) #where_clause {
			world
				.commands()
				.entity(cx.entity)
			  #(.observe_any(#observers))*;
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

#[cfg(test)]
mod test {
	use super::parse;
	use quote::quote;
	use sweet::prelude::*;
	use syn::DeriveInput;

	#[test]
	fn works() {
		let input: DeriveInput = syn::parse_quote! {
			#[derive(Component)]
			struct Foo;
		};
		parse(
			// equivelent to [action(bar,baz)]
			quote!(bar, bazz),
			input,
		)
		.unwrap()
		.xpect_snapshot();
	}
	#[test]
	fn generics() {
		let input: DeriveInput = syn::parse_quote! {
			#[derive(Component)]
			struct Foo<T>;
		};

		parse(
			// equivelent to [action(bar,baz::<T>)]
			quote!(bar, bazz::<T>),
			input,
		)
		.unwrap()
		.xpect_snapshot();
	}
}

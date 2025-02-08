use proc_macro;
use proc_macro::TokenStream;
use proc_macro2;
use quote::quote;
use syn;
use syn::Path;
use syn::Type;
use syn::TypePath;

pub fn derive_deref(input: TokenStream) -> TokenStream {
	let item = syn::parse(input).unwrap();
	let (field_ty, field_access) = parse_fields(&item, false);

	let name = &item.ident;
	let (impl_generics, ty_generics, where_clause) =
		item.generics.split_for_impl();

	quote!(
		impl #impl_generics core::ops::Deref for #name #ty_generics
		#where_clause
		{
			type Target = #field_ty;

			fn deref(&self) -> &Self::Target {
				#field_access
			}
		}
	)
	.into()
}

pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
	let item = syn::parse(input).unwrap();
	let (_, field_access) = parse_fields(&item, true);

	let name = &item.ident;
	let (impl_generics, ty_generics, where_clause) =
		item.generics.split_for_impl();

	quote!(
		impl #impl_generics core::ops::DerefMut for #name #ty_generics
		#where_clause
		{
			fn deref_mut(&mut self) -> &mut Self::Target {
				#field_access
			}
		}
	)
	.into()
}

fn parse_fields(
	item: &syn::DeriveInput,
	mutable: bool,
) -> (syn::Type, proc_macro2::TokenStream) {
	let trait_name = if mutable { "DerefMut" } else { "Deref" };
	let fields = match item.data {
		syn::Data::Struct(ref body) => {
			body.fields.iter().collect::<Vec<&syn::Field>>()
		}
		_ => panic!("#[derive({})] can only be used on structs", trait_name),
	};

	let field_ty = match fields.len() {
		1 => Some(fields[0].ty.clone()),
		2 => {
			if let Type::Path(TypePath {
				path: Path { segments, .. },
				..
			}) = &fields[1].ty
			{
				if segments
					.last()
					.expect("Expected path to have at least one segment")
					.ident == "PhantomData"
				{
					Some(fields[0].ty.clone())
				} else {
					None
				}
			} else {
				None
			}
		}
		_ => None,
	};
	let field_ty = field_ty.unwrap_or_else(|| {
		panic!(
			"#[derive({})] can only be used on structs with one field, \
                 and optionally a second `PhantomData` field.",
			trait_name,
		)
	});

	let field_name = match fields[0].ident {
		Some(ref ident) => quote!(#ident),
		_ => quote!(0),
	};

	match (field_ty, mutable) {
		(syn::Type::Reference(syn::TypeReference { elem, .. }), _) => {
			(*elem.clone(), quote!(self.#field_name))
		}
		(x, true) => (x, quote!(&mut self.#field_name)),
		(x, false) => (x, quote!(&self.#field_name)),
	}
}

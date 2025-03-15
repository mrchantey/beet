mod node_field;
use node_field::NodeField;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::Result;
use syn::Type;

pub fn impl_derive_node(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = match input.data {
		Data::Struct(ref data) => match data.fields {
			Fields::Named(ref fields) => &fields.named,
			_ => {
				return Err(syn::Error::new_spanned(
					&input,
					"Only named fields are supported",
				));
			}
		},
		_ => {
			return Err(syn::Error::new_spanned(
				&input,
				"Only structs are supported",
			));
		}
	}
	.iter()
	.map(|f| NodeField::parse(f))
	.collect::<Result<Vec<_>>>()?;


	let impl_node = impl_node(&input)?;
	let impl_builder = impl_builder(&input, &fields)?;
	let impl_required = impl_required(&input, &fields)?;

	Ok(quote! {
		#impl_node
		#impl_builder
		#impl_required
	})
}



fn impl_node(input: &DeriveInput) -> Result<TokenStream> {
	let name = &input.ident;
	let impl_builder_name = format_ident!("{}Builder", &input.ident);
	let impl_required_name = format_ident!("{}Required", &input.ident);

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {

		impl #impl_generics Node for #name #type_generics #where_clause {
			type Builder = #impl_builder_name #type_generics;
			type Required = #impl_required_name;

			fn into_rsx(self) -> RsxRoot {
				// todo use attribute
				into_rsx(self)
			}
		}


		// #impl_component
	})
}


fn impl_builder(
	input: &DeriveInput,
	fields: &[NodeField],
) -> Result<TokenStream> {
	let builder_fields = fields.iter().map(|field| {
		let name = &field.inner.ident;
		let ty = get_inner_type(&field.inner.ty);
		if field.attribute("default").is_some() {
			quote! { #name: #ty }
		} else {
			quote! { #name: Option<#ty> }
		}
	});

	let default_fallback = syn::parse_quote! { Default::default() };

	let builder_defaults = fields.iter().map(|field| {
		let name = &field.inner.ident;
		if let Some(attr) = field.attribute("default") {
			let val = attr.value.as_ref().unwrap_or(&default_fallback);
			quote! { #name: #val }
		} else {
			quote! { #name: None }
		}
	});


	let set_val_methods = fields.iter().map(|field| {
		let name = &field.inner.ident;
		let ty = get_inner_type(&field.inner.ty);

		let rhs = if field.attribute("default").is_some() {
			quote! { value }
		} else {
			quote! { Some(value) }
		};
		quote! {
			fn #name(mut self, value: #ty) -> Self {
				self.#name = #rhs;
				self
			}
		}
	});
	let unwrap_fields = fields.iter().map(|field| {
		let name = &field.inner.ident;

		let rhs = if field.attribute("default").is_some() {
			quote! { self.#name }
		} else if field.is_optional() {
			quote! { self.#name }
		} else {
			quote! { self.#name.unwrap() }
		};
		quote! {#name: #rhs}
	});

	let node_name = &input.ident;
	let impl_builder_name = format_ident!("{}Builder", &input.ident);
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let vis = &input.vis;

	Ok(quote! {

		#vis struct #impl_builder_name #impl_generics {
			#(#builder_fields),*
		}

		impl #impl_generics #impl_builder_name #type_generics #where_clause {
			#(#set_val_methods)*
		}

		impl #impl_generics Default for #impl_builder_name #type_generics #where_clause {
			fn default() -> Self {
				Self {
					#(#builder_defaults),*
				}
			}
		}

		impl #impl_generics NodeBuilder for #impl_builder_name #type_generics #where_clause {
			type Node = #node_name #type_generics;

			fn build(self) -> Self::Node {
				Self::Node{
					#(#unwrap_fields),*
				}
			}
		}
	})
}

fn impl_required(
	input: &DeriveInput,
	fields: &[NodeField],
) -> Result<TokenStream> {
	let required_field_names = fields.iter().filter_map(|field| {
		if field.is_required() {
			Some(&field.inner.ident)
		} else {
			None
		}
	});

	let impl_required_name = format_ident!("{}Required", &input.ident);
	let vis = &input.vis;

	Ok(quote! {
		#vis struct #impl_required_name {
			#(#required_field_names: bool),*
		}
	})
}


/// Returns the inner type of a type, unwrapping Option<T> to T.
fn get_inner_type(ty: &Type) -> &Type {
	if let Type::Path(p) = ty {
		if let Some(segment) = p.path.segments.last() {
			if segment.ident == "Option" {
				if let syn::PathArguments::AngleBracketed(args) =
					&segment.arguments
				{
					if let Some(syn::GenericArgument::Type(ty)) =
						args.args.first()
					{
						return ty;
					}
				}
			}
		}
	}
	ty
}

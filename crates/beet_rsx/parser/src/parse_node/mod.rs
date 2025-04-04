mod props_field;
use crate::prelude::*;
use proc_macro2::TokenStream;
use props_field::*;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Expr;
use syn::Fields;
use syn::Ident;
use syn::Result;
use syn::Type;
use syn::parse_quote_spanned;
use syn::spanned::Spanned;

pub fn impl_derive_node(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let unit_fields = Default::default();
	let fields = match input.data {
		Data::Struct(ref data) => match &data.fields {
			Fields::Unit => &unit_fields,
			Fields::Named(fields) => &fields.named,
			Fields::Unnamed(_) => {
				return Err(syn::Error::new_spanned(
					&input,
					"Unnamed structs are not currently supported",
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
	.map(|f| PropsField::parse(f))
	.collect::<Result<Vec<_>>>()?;

	let impl_component = impl_component(&input)?;
	let impl_props = impl_props(&input)?;
	let impl_builder = impl_builder(&input, &fields)?;
	let impl_required = impl_required(&input, &fields)?;

	Ok(quote! {
		use beet::prelude::*;

		#impl_component
		#impl_props
		#impl_builder
		#impl_required
	})
}

fn impl_component(input: &DeriveInput) -> Result<TokenStream> {
	let attributes = AttributeGroup::parse(&input.attrs, "node")?;
	if attributes.get("no_component").is_some() {
		return Ok(Default::default());
	}

	let into_rsx = if let Some(into_rsx) = attributes.get("into_rsx") {
		into_rsx
			.value
			.as_ref()
			.map(|expr| expr.to_token_stream())
			.unwrap_or_else(|| {
				Expr::Verbatim(quote! { into_rsx }).to_token_stream()
			})
	} else {
		Ident::new(
			&heck::AsSnakeCase(&input.ident.to_string()).to_string(),
			input.ident.span(),
		)
		.to_token_stream()
	};

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let name = &input.ident;

	Ok(quote! {
	impl #impl_generics beet::prelude::Component for #name #type_generics #where_clause {

		fn render(self) -> RsxRoot {
			#into_rsx(self)
		}
	}
	})
}


fn impl_props(input: &DeriveInput) -> Result<TokenStream> {
	let name = &input.ident;
	let impl_builder_name = format_ident!("{}Builder", &input.ident);
	let impl_required_name = format_ident!("{}Required", &input.ident);

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {
		impl #impl_generics Props for #name #type_generics #where_clause {
			type Builder = #impl_builder_name #type_generics;
			type Required = #impl_required_name;
		}


		// #impl_component
	})
}


fn impl_builder(
	input: &DeriveInput,
	fields: &[PropsField],
) -> Result<TokenStream> {
	let builder_fields = fields.iter().map(|field| {
		let name = &field.inner.ident;
		let ty = get_inner_type(&field.inner.ty);
		if field.default_attr().is_some() {
			quote! { #name: #ty }
		} else {
			quote! { #name: Option<#ty> }
		}
	});

	let default_fallback = syn::parse_quote! { Default::default() };

	let builder_defaults = fields.iter().map(|field| {
		let name = &field.inner.ident;
		if let Some(attr) = field.default_attr() {
			let val = attr.value.as_ref().unwrap_or(&default_fallback);
			quote! { #name: #val }
		} else {
			quote! { #name: None }
		}
	});


	let set_val_methods = fields.iter().map(|field| {
		let name = &field.inner.ident;
		let ty = get_inner_type(&field.inner.ty);
		let is_into = field.attributes.get("into").is_some();
		let value = if is_into {
			quote! { value.into() }
		} else {
			quote! { value }
		};

		let ty = if is_into {
			parse_quote_spanned! { ty.span() => impl Into<#ty> }
		} else {
			ty.to_token_stream()
		};

		let rhs = if field.default_attr().is_some() {
			quote! { #value }
		} else {
			quote! { Some(#value) }
		};
		quote! {
			#[allow(missing_docs)]
			pub fn #name(mut self, value: #ty) -> Self {
				self.#name = #rhs;
				self
			}
		}
	});
	let unwrap_fields = fields.iter().map(|field| {
		let name = &field.inner.ident;

		let rhs = if field.default_attr().is_some() {
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
		#[allow(missing_docs)]
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

		impl #impl_generics PropsBuilder for #impl_builder_name #type_generics #where_clause {
			type Component = #node_name #type_generics;

			fn build(self) -> Self::Component {
				Self::Component{
					#(#unwrap_fields),*
				}
			}
		}
	})
}

fn impl_required(
	input: &DeriveInput,
	fields: &[PropsField],
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
		#[allow(missing_docs)]
		#[derive(Default)]
		#vis struct #impl_required_name {
			#(pub #required_field_names: bool),*
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	#[ignore = "too flaky, we need smaller tests"]
	fn works() {
		let input = syn::parse_quote! {
			#[derive(Node)]
			#[node(into_rsx = my_node)]
			struct MyNode {
				is_required: u32,
				is_optional: Option<u32>,
				#[field(default = 7)]
				is_default: u32,
				#[field(default)]
				is_generic_default: Foo<u32>,
			}
		};


		let expected = quote! {
			use beet::prelude::*;

			impl beet::prelude::Component for MyNode {
				fn render (self) -> RsxRoot { my_node (self) }
			}
			impl Props for MyNode {
				type Builder = MyNodeBuilder;
				type Required = MyNodeRequired;
			}
			#[allow(missing_docs)]
			struct MyNodeBuilder {
				is_required: Option<u32>,
				is_optional: Option<u32>,
				is_default: u32,
				is_generic_default: Foo<u32>
			}

			impl MyNodeBuilder {
				#[allow(missing_docs)]
				pub fn is_required(mut self, value: u32) -> Self {
					self.is_required = Some(value);
					self
				}

				#[allow(missing_docs)]
				pub fn is_optional(mut self, value: u32) -> Self {
					self.is_optional = Some(value);
					self
				}

				#[allow(missing_docs)]
				pub fn is_default(mut self, value: u32) -> Self {
					self.is_default = value;
					self
				}

				#[allow(missing_docs)]
				pub fn is_generic_default(mut self, value: Foo<u32>) -> Self {
					self.is_generic_default = value;
					self
				}
			}

			impl Default for MyNodeBuilder {
				fn default() -> Self {
					Self {
						is_required: None,
						is_optional: None,
						is_default: 7,
						is_generic_default: Default::default()
					}
				}
			}

			impl PropsBuilder for MyNodeBuilder {
				type Component = MyNode;

				fn build(self) -> Self::Component {
					Self::Component {
						is_required: self.is_required.unwrap(),
						is_optional: self.is_optional,
						is_default: self.is_default,
						is_generic_default: self.is_generic_default
					}
				}
			}

			#[allow(missing_docs)]
			struct MyNodeRequired {
				pub is_required: bool
			}
		};

		let actual = impl_derive_node(input);
		expect(actual.to_string()).to_be(expected.to_string());
	}
}

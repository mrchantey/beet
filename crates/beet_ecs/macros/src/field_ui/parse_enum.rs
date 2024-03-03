use crate::parse_field_attrs;
use crate::utils::*;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::DataEnum;
use syn::Field;
use syn::Result;
use syn::Variant;

pub fn parse_enum(
	input: DataEnum,
	enum_field_variant: Option<TokenStream>,
) -> Result<TokenStream> {
	let is_hidden = enum_field_variant.is_none();

	let variants = input
		.variants
		.iter()
		.map(|v| parse_enum_variant(v, is_hidden))
		.collect::<Result<Vec<_>>>()?
		.into_iter()
		.collect_comma_punct();

	// input.enum_token

	let parent_def = if is_hidden {
		TokenStream::new()
	} else {
		// TODO use enum_field_variant
		quote! {
			let select = SelectField::new(
				reflect.field_name.clone(),
				reflect.clone_get_cb(),
				reflect.clone_set_cb(),
			);
		}
	};

	Ok(quote! {
		#parent_def
		let val = reflect.get();
		#[allow(unused_variables)]
		match val{
			#variants
		}
	})
}


fn parse_enum_variant(
	variant: &Variant,
	parent_is_hidden: bool,
) -> Result<TokenStream> {
	let variant_ident = &variant.ident;
	let parent_iter_val = if parent_is_hidden {
		TokenStream::new()
	} else {
		quote! {select.into(),}
	};

	let out = match &variant.fields {
		syn::Fields::Unit => {
			if parent_is_hidden {
				quote! {Self::#variant_ident => HeadingField::new("No Fields".to_string()).into()}
			// quote! {Self::#variant_ident => select.into()}
			} else {
				quote! {Self::#variant_ident => select.into()}
			}
		}
		syn::Fields::Unnamed(fields) => {
			let field_idents = unnamed_field_idents(fields);
			let variant_with_fields =
				quote! {Self::#variant_ident(#field_idents)};
			let fields = fields
				.unnamed
				.iter()
				.enumerate()
				.map(|(i, field)| {
					let mut field = field.clone();
					field.ident = Some(field_ident(i, &field));
					parse_enum_field(&field, &variant_with_fields)
				})
				.collect_tokens()?;

			quote! {
				Self::#variant_ident(#field_idents) =>
				GroupField::new(reflect.display_name.clone(), vec![
				#parent_iter_val #fields
			]).into()
			}
		}
		syn::Fields::Named(fields) => {
			let field_idents = fields
				.named
				.iter()
				.map(|f| f.ident.to_token_stream())
				.collect_comma_punct();
			let variant_with_fields =
				quote! {Self::#variant_ident{#field_idents}};
			let fields = fields
				.named
				.iter()
				.map(|field| parse_enum_field(&field, &variant_with_fields))
				.collect_tokens()?;
			quote! {variant_with_fields =>
					GroupField::new(reflect.display_name.clone(), vec![
					#parent_iter_val #fields
				]).into()
			}
		}
	};

	Ok(out)
}

fn unnamed_field_idents(fields: &syn::FieldsUnnamed) -> TokenStream {
	fields
		.unnamed
		.iter()
		.enumerate()
		.map(|(i, field)| field_ident(i, field).into_token_stream())
		.collect_comma_punct()
}

fn field_ident(index: usize, field: &Field) -> Ident {
	Ident::new(&format!("field{index}"), field.span())
}

const ERROR: &str = "Unexpected enum variant, usually because UI was not recreated after the enum changed";
fn parse_enum_field(
	field: &Field,
	variant_with_fields: &TokenStream,
) -> Result<Option<TokenStream>> {
	let field_ident = field
		.ident
		.as_ref()
		.expect("shouldnt happen, ive set unnamed manually");
	let ident_str = field_ident.to_string();

	let reflect = quote! {
		{
			let checked_get = {
				let get_cb = reflect.clone_get_cb();
				move || match get_cb() {
					#[allow(unused_variables)]
					#variant_with_fields => #field_ident,
					_ => panic!(#ERROR),
				}
			};
			let checked_set = {
				let get_cb = reflect.clone_get_cb();
				let set_cb = reflect.clone_set_cb();
				move |val| match get_cb() {
					#[allow(unused_variables)]
					#variant_with_fields => {
						let #field_ident = val;
						set_cb(#variant_with_fields);
					},
					_ => panic!(#ERROR),
				}
			};
			FieldReflect::new(
				#ident_str.to_string(),
				checked_get,
				checked_set,
			)
		}
	};

	Ok(parse_field_attrs(field, &reflect)?)
}

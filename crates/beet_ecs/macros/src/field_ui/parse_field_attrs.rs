use crate::utils::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use std::collections::HashMap;
use syn::Attribute;
use syn::Expr;
use syn::Field;
use syn::Ident;
use syn::Result;


pub fn parse_field_attrs(
	field: &Field,
	reflect: &TokenStream,
) -> Result<Option<TokenStream>> {
	parse_type_attrs(&field.ty.to_token_stream(), &field.attrs, reflect)
}
pub fn parse_type_attrs(
	ty: &TokenStream,
	attrs: &Vec<Attribute>,
	reflect: &TokenStream,
) -> Result<Option<TokenStream>> {
	let out = match get_variant(attrs)? {
		FieldUiVariant::FromType => Some(quote! {#ty::into_field_ui(#reflect)}),
		FieldUiVariant::FieldAttrs(FieldAttrs { ident, fields }) => {
			Some(quote! {
				#ident::<#ty>{
					reflect: #reflect,
					#fields,
					..Default::default()
				}.into()
			})
		}
		FieldUiVariant::None => None,
	};
	Ok(out)
}
enum FieldUiVariant {
	None,
	FromType,
	FieldAttrs(FieldAttrs),
}

struct FieldAttrs {
	ident: Ident,
	fields: TokenStream,
}
impl FieldAttrs {
	pub fn new(
		ident: &'static str,
		field_names: &[&'static str],
		args: HashMap<String, Option<Expr>>,
	) -> Self {
		let ident = Ident::new(ident, Span::call_site());
		let fields = field_names
			.iter()
			.filter_map(|name| {
				args.get(*name).and_then(|val| val.clone()).map(|val| {
					let ident = Ident::new(name, Span::call_site());
					quote!(#ident: #val)
				})
			})
			.collect_comma_punct();
		Self { ident, fields }
	}
}

fn get_variant(attrs: &Vec<Attribute>) -> Result<FieldUiVariant> {
	for attr in attrs.iter() {
		let args: TokenStream = attr.parse_args().unwrap_or_default();
		let args = attributes_map(args, None)?;

		let out = if attr
			.meta
			.path()
			.is_ident(&Ident::new("hide_ui", Span::call_site()))
		{
			Some(FieldUiVariant::None)
		} else if attr
			.meta
			.path()
			.is_ident(&Ident::new("number", Span::call_site()))
		{
			Some(FieldUiVariant::FieldAttrs(FieldAttrs::new(
				"NumberField",
				&["min", "max", "step", "display"],
				args,
			)))
		} else {
			None
		};
		if let Some(out) = out {
			return Ok(out);
		}
	}
	Ok(FieldUiVariant::FromType)
}
// .ok_or_else(|| {
// 	syn::Error::new(
// 		attr.span(),
// 		format!(
// 			"{attr_name} attribute must have a '{name}' arg"
// 		),
// 	)
// })?
// .clone()
// .ok_or_else(|| {
// 	syn::Error::new(
// 		attr.span(),
// 		format!(
// 			"{attr_name} attribute must have a '{name}' arg"
// 		),
// 	)
// })?)

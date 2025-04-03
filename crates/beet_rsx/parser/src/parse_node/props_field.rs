//! copied from [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui-derive/src/attributes.rs)
use crate::prelude::*;
use syn::Data;
use syn::DeriveInput;
use syn::Expr;
use syn::Field;
use syn::Fields;
use syn::Result;
use syn::Type;

#[derive(Debug)]
pub struct PropsField<'a> {
	pub inner: &'a Field,
	pub attributes: AttributeGroup,
}


impl<'a> PropsField<'a> {
	pub fn parse_all(input: &'a DeriveInput) -> Result<Vec<PropsField<'a>>> {
		match &input.data {
			Data::Struct(data) => match &data.fields {
				Fields::Unit => return Ok(Default::default()),
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
		.collect::<Result<Vec<_>>>()
	}

	pub fn parse(inner: &'a Field) -> Result<Self> {
		let attributes = AttributeGroup::parse(&inner.attrs, "field")?
			.validate_allowed_keys(&[
				"default", "required", "into", "no_into", "flatten",
			])?;
		Ok(Self { inner, attributes })
	}

	pub fn is_optional(&self) -> bool {
		matches!(self.inner.ty, syn::Type::Path(ref p) if p.path.segments.last()
				.map(|s| s.ident == "Option")
				.unwrap_or(false))
	}

	/// if the attribute has the default or flatten (which implies default) fields
	pub fn is_default(&self) -> bool {
		self.attributes.contains("default")
			|| self.attributes.contains("flatten")
	}

	/// Returns true if the field is required.
	pub fn is_required(&self) -> bool {
		self.is_optional() == false
			&& self.attributes.contains("default") == false
	}

	pub fn default_attr(&self) -> Option<&AttributeItem> {
		self.attributes.get("default")
	}
	/// Returns the inner type of a type, unwrapping Option<T> to T.
	pub fn unwrap_type(&self) -> &Type {
		if let Type::Path(p) = &self.inner.ty {
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
		&self.inner.ty
	}

	pub fn is_into(&self) -> bool {
		if self.attributes.contains("into") {
			return true;
		} else if self.attributes.contains("no_into") {
			return false;
		} else if self.inner.ty == syn::parse_quote! { String } {
			return true;
		} else {
			return false;
		}
	}

	/// In Builder pattern these are the tokens for assignment, depending
	/// on attributes it may be one fof the following:
	/// - `(SomeVal, value)`
	/// - `(impl Into<SomeVal>, value.into())`
	pub fn assign_tokens(&self) -> (Type, Expr) {
		let is_into = self.is_into();
		let inner_ty = self.unwrap_type();
		match is_into {
			true => (
				syn::parse_quote! { impl Into<#inner_ty> },
				syn::parse_quote! { value.into() },
			),
			false => {
				(syn::parse_quote! { #inner_ty }, syn::parse_quote! { value })
			}
		}
	}
}

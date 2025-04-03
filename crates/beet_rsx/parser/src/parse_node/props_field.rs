//! copied from [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui-derive/src/attributes.rs)
use crate::prelude::*;
use syn::Data;
use syn::DeriveInput;
use syn::Expr;
use syn::Field;
use syn::Fields;
use syn::Meta;
use syn::Result;
use syn::Type;

#[derive(Debug)]
pub struct PropsField<'a> {
	pub inner: &'a Field,
	/// The inner type of the field, unwrapping Option<T> to T.
	pub unwrapped: &'a Type,
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



		Ok(Self {
			unwrapped: Self::unwrap_type(inner),
			inner,
			attributes,
		})
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
	pub fn unwrap_type(field: &Field) -> &Type {
		if let Type::Path(p) = &field.ty {
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
		&field.ty
	}

	pub fn is_into(&self) -> bool {
		if self.attributes.contains("into") {
			return true;
		} else if self.attributes.contains("no_into") {
			return false;
		} else if self.unwrapped == &syn::parse_quote! { String } {
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
		let inner_ty = self.unwrapped;
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

	/// Create a new [`Documentation`] from a type's attributes.
	///
	/// This will collect all `#[doc = "..."]` attributes, including the ones generated via `///` and `//!`.
	pub fn docs(&self) -> Vec<&syn::Attribute> {
		self.inner
			.attrs
			.iter()
			.filter_map(|attr| match &attr.meta {
				Meta::NameValue(pair) if pair.path.is_ident("doc") => {
					Some(attr)
				}
				_ => None,
			})
			.collect()
	}
}

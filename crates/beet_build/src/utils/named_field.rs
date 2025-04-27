use crate::prelude::*;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Field;
use syn::Fields;
use syn::GenericArgument;
use syn::Ident;
use syn::Meta;
use syn::PathArguments;
use syn::PathSegment;
use syn::Result;
use syn::Type;
use syn::parse_quote;

/// A wrapper around [`syn::Field`] that provides additional functionality
#[derive(Debug)]
pub struct NamedField<'a> {
	pub syn_field: &'a Field,
	/// The `Bar` in `foo: Bar` or `foo: Option<Bar>`
	pub inner_ty: &'a Type,
	/// The `(Bar,Bazz)` in `foo: Bar<Bazz>` or `foo: Option<Bar<Bazz>>`
	pub inner_generics: Option<(&'a PathSegment, &'a Type)>,
	/// The `foo` in `foo: Bar`
	/// Only named fields are supported
	pub ident: &'a Ident,
	pub attributes: AttributeGroup,
}


impl<'a> NamedField<'a> {
	pub fn parse_all(input: &'a DeriveInput) -> Result<Vec<NamedField<'a>>> {
		match &input.data {
			Data::Struct(data) => match &data.fields {
				Fields::Unit => return Ok(Default::default()),
				Fields::Named(fields) => &fields.named,
				Fields::Unnamed(_) => {
					return Err(Error::new_spanned(
						&input,
						"Unnamed structs are not currently supported",
					));
				}
			},
			_ => {
				return Err(Error::new_spanned(
					&input,
					"Only structs are supported",
				));
			}
		}
		.iter()
		.map(|f| NamedField::parse(f))
		.collect::<Result<Vec<_>>>()
	}

	pub fn parse(inner: &'a Field) -> Result<Self> {
		let attributes = AttributeGroup::parse(&inner.attrs, "field")?;
		let ident = inner.ident.as_ref().ok_or_else(|| {
			Error::new_spanned(inner, "Only named fields are supported")
		})?;

		let inner_ty = Self::option_inner(inner);

		Ok(Self {
			inner_generics: Self::generic_inner(inner_ty),
			inner_ty,
			ident,
			// ident: &inner.ident,
			syn_field: inner,
			attributes,
		})
	}

	/// Returs whether this field is of type `Option<T>`.
	pub fn is_optional(&self) -> bool {
		matches!(self.syn_field.ty, Type::Path(ref p) if p.path.segments.last()
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

	/// 1. First checks for a specified attribute
	/// By default strings are converted to `impl Into<String>`.
	pub fn is_into(&self) -> bool {
		if self.attributes.contains("into") {
			return true;
		} else if self.attributes.contains("no_into") {
			return false;
		} else if self.inner_ty == &parse_quote! { String } {
			return true;
		} else {
			return false;
		}
	}

	/// Returns the inner type of an Option, unwrapping Option<T> to T.
	fn option_inner(field: &Field) -> &Type {
		if let Type::Path(p) = &field.ty {
			if let Some(segment) = p.path.segments.last() {
				if segment.ident == "Option" {
					if let PathArguments::AngleBracketed(args) =
						&segment.arguments
					{
						if let Some(GenericArgument::Type(ty)) =
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

	/// Returns the containing and inner types of a generic.
	/// If the field type is an Option<Foo<Bar>>, this will
	/// return (Foo, Bar).
	fn generic_inner(ty: &Type) -> Option<(&PathSegment, &Type)> {
		if let Type::Path(p) = ty {
			if let Some(segment) = p.path.segments.last() {
				if let PathArguments::AngleBracketed(args) = &segment.arguments
				{
					if let Some(GenericArgument::Type(ty)) = args.args.first() {
						return Some((segment, ty));
					}
				}
			}
		}
		None
	}

	/// Create a new [`Documentation`] from a type's attributes.
	///
	/// This will collect all `#[doc = "..."]` attributes, including the ones generated via `///` and `//!`.
	pub fn docs(&self) -> Vec<&Attribute> {
		self.syn_field
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

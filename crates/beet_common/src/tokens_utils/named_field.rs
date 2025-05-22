use crate::prelude::*;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Field;
use syn::Fields;
use syn::GenericArgument;
use syn::Ident;
use syn::ItemFn;
use syn::Meta;
use syn::Pat;
use syn::PatIdent;
use syn::PatType;
use syn::PathArguments;
use syn::PathSegment;
use syn::Result;
use syn::Type;
use syn::parse_quote;

/// A wrapper around a struct [`syn::Field`] or function input [`PatType`]
/// that provides additional functionality
#[derive(Debug)]
pub struct NamedField<'a> {
	pub attrs: &'a Vec<Attribute>,
	/// The `Bar` in `foo: Bar`
	pub ty: &'a Type,
	/// The `Bar` in `foo: Option<Bar>` or `foo: Bar`
	pub inner_ty: &'a Type,
	/// The `(Bar,Bazz)` in `foo: Bar<Bazz>` or `foo: Option<Bar<Bazz>>`
	pub inner_generics: Option<(&'a PathSegment, &'a Type)>,
	/// The `foo` in `foo: Bar`
	/// Only named fields are supported
	pub ident: &'a Ident,
	/// Attributes under the 'field' key,
	/// ie `#[field(default)]`
	pub field_attributes: AttributeGroup,
}

impl<'a> NamedField<'a> {
	pub fn parse_item_fn(input: &'a ItemFn) -> Result<Vec<NamedField<'a>>> {
		input
			.sig
			.inputs
			.iter()
			.filter_map(|arg| {
				if let syn::FnArg::Typed(pat) = arg {
					Some(pat)
				} else {
					None
				}
			})
			.map(Self::parse_pat_ty)
			.collect::<Result<Vec<_>>>()
	}
	pub fn parse_derive_input(
		input: &'a DeriveInput,
	) -> Result<Vec<NamedField<'a>>> {
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
		.map(|f| Self::parse_field(f))
		.collect::<Result<Vec<_>>>()
	}

	pub fn parse_pat_ty(inner: &'a PatType) -> Result<Self> {
		let attributes = AttributeGroup::parse(&inner.attrs, "field")?;
		let Pat::Ident(PatIdent { ident, .. }) = &*inner.pat else {
			return Err(Error::new_spanned(
				inner,
				"Only named fields are supported",
			));
		};

		Ok(Self {
			inner_generics: Self::generic_inner(&inner.ty),
			inner_ty: Self::option_inner(&inner.ty),
			ty: &inner.ty,
			attrs: &inner.attrs,
			ident,
			field_attributes: attributes,
		})
	}
	pub fn parse_field(inner: &'a Field) -> Result<Self> {
		let attributes = AttributeGroup::parse(&inner.attrs, "field")?;
		let ident = inner.ident.as_ref().ok_or_else(|| {
			Error::new_spanned(inner, "Only named fields are supported")
		})?;

		Ok(Self {
			inner_generics: Self::generic_inner(&inner.ty),
			inner_ty: Self::option_inner(&inner.ty),
			ty: &inner.ty,
			attrs: &inner.attrs,
			ident,
			field_attributes: attributes,
		})
	}

	/// Returs whether this field is of type `Option<T>`.
	pub fn is_optional(&self) -> bool {
		matches!(self.ty, Type::Path(p) if p.path.segments.last()
				.map(|s| s.ident == "Option")
				.unwrap_or(false))
	}

	/// if the attribute has the default or flatten (which implies default) fields
	pub fn is_default(&self) -> bool {
		self.field_attributes.contains("default")
			|| self.field_attributes.contains("flatten")
	}

	/// Returns true if the field is required.
	pub fn is_required(&self) -> bool {
		self.is_optional() == false
			&& self.field_attributes.contains("default") == false
	}

	/// 1. First checks for a specified attribute
	/// By default strings are converted to `impl Into<String>`.
	pub fn is_into(&self) -> bool {
		if self.field_attributes.contains("into") {
			return true;
		} else if self.field_attributes.contains("no_into") {
			return false;
		} else if self.inner_ty == &parse_quote! { String } {
			return true;
		} else {
			return false;
		}
	}

	/// Returns the inner type of an Option, unwrapping Option<T> to T.
	fn option_inner(ty: &Type) -> &Type {
		if let Type::Path(p) = ty {
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
		ty
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
		self.attrs
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	use syn::FnArg;

	#[test]
	fn fields() {
		let field = syn::parse_quote! {
			#[field(default)]
			pub foo: Option<u32>
		};
		let named = NamedField::parse_field(&field).unwrap();
		expect(named.is_optional()).to_be_true();
		expect(named.attrs.len()).to_be(1);
		// expect(true).to_be_false();
	}
	#[test]
	fn pat_ty() {
		let field = syn::parse_quote! {
			#[field(default)]
			foo: Option<u32>
		};
		let FnArg::Typed(field) = field else {
			panic!();
		};

		let named = NamedField::parse_pat_ty(&field).unwrap();
		expect(named.is_optional()).to_be_true();
		expect(named.attrs.len()).to_be(1);
	}
}

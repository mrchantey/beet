use beet_common::prelude::*;
use proc_macro2::TokenStream;
use syn::DeriveInput;
use syn::Expr;
use syn::ItemFn;
use syn::Result;
use syn::Type;
use syn::parse_quote;


/// A wrapper around [`NamedField`] that provides additional functionality
/// for fields of a `Node` or Builder.
pub struct NodeField<'a>(NamedField<'a>);

impl<'a> std::ops::Deref for NodeField<'a> {
	type Target = NamedField<'a>;
	fn deref(&self) -> &Self::Target { &self.0 }
}


impl<'a> NodeField<'a> {
	pub fn parse_item_fn(input: &ItemFn) -> Result<Vec<NodeField>> {
		NamedField::parse_item_fn(input)?
			.into_iter()
			.map(|field| {
				Self::validate_keys(&field)?;
				Ok(NodeField(field))
			})
			.collect()
	}
	pub fn parse_derive_input(input: &DeriveInput) -> Result<Vec<NodeField>> {
		NamedField::parse_derive_input(input)?
			.into_iter()
			.map(|field| {
				Self::validate_keys(&field)?;
				Ok(NodeField(field))
			})
			.collect()
	}
	fn validate_keys(named_field: &NamedField) -> Result<()> {
		named_field.field_attributes.validate_allowed_keys(&[
			"default",
			"required",
			"into",
			"no_into",
			"into_generics",
			"into_func",
			"into_type",
			"flatten",
		])?;
		Ok(())
	}


	/// In Builder pattern these are the tokens for assignment, depending
	/// on attributes it will be checked in the following order:
	/// - is_boxed:				`(Default, 				impl SomeType, 					Box::new(value))`
	/// - EventHandler<E>:`(<E,B,M>, 				impl IntoObserverSystem<E, B, M>,,		EventHandler::new(value)`
	/// - MaybeSignal<T>:	`(<M>, 						impl IntoMaybeSignal,		value.into_maybe_signal())`
	/// - into_type:			`(into_generics,	into_type, into_func							)`
	/// - is_into: 				`(Default, 				impl Into<SomeType>, 		value.into())		`
	/// - verbatim: 			`(Default, 				SomeType, 							value)					`
	pub fn assign_tokens(
		field: &NamedField,
	) -> Result<(TokenStream, Type, Expr)> {
		match field.inner_generics {
			// 1. handle boxed trait objects
			Some((seg, Type::TraitObject(obj))) if seg.ident == "Box" => {
				let mut trait_bounds = obj.bounds.clone();
				trait_bounds.push(parse_quote! { 'static });
				Ok((
					TokenStream::new(),
					parse_quote! { impl #trait_bounds },
					parse_quote! { Box::new(value) },
				))
			}
			// 2. handle EventHandler<T>
			Some((seg, ty)) if seg.ident == "EventHandler" => Ok((
				parse_quote! {<B:Bundle,M>},
				parse_quote! { impl bevy::ecs::system::IntoObserverSystem<#ty,B,M> },
				parse_quote! { EventHandler::new(value) },
			)),
			// 2. handle MaybeSignal<T>
			Some((seg, ty)) if seg.ident == "MaybeSignal" => Ok((
				parse_quote! {<M>},
				parse_quote! { impl beet::prelude::IntoMaybeSignal<#ty,M> },
				parse_quote! { value.into_maybe_signal() },
			)),
			// 3. handle into_type attribute
			_ if let Some(ty) = field
				.field_attributes
				.get_value_parsed::<Type>("into_type")? =>
			{
				let generics = field
					.field_attributes
					.get_value_parsed::<TokenStream>("into_generics")?
					.unwrap_or_default();

				let func = field
					.field_attributes
					.get_value_parsed::<Expr>("into_func")?
					.map(|func| {
						parse_quote! { value.#func() }
					})
					.unwrap_or_else(|| {
						parse_quote! { value.into() }
					});

				// this is wrong..
				// why is this wrong?
				return Ok((generics, ty, func));
			}
			// 4. handle the rest
			_ => {
				let is_into = field.is_into();
				let inner_ty = field.inner_ty;
				match is_into {
					true => Ok((
						TokenStream::new(),
						parse_quote! { impl Into<#inner_ty> },
						parse_quote! { value.into() },
					)),
					// 3. verbatim
					false => Ok((
						TokenStream::new(),
						parse_quote! { #inner_ty },
						parse_quote! { value },
					)),
				}
			}
		}
	}
}

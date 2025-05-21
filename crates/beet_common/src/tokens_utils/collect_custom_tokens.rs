use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use sweet::prelude::PipelineTarget;


/// A trait that can be implemented for types that implement [`SystemParam`]
pub trait CollectCustomTokens {
	/// For each [`MaybeSpannedQuery`] in this struct, push the token stream
	/// for that component if it is present.
	fn try_push_all(
		&self,
		spans: &NonSendAssets<Span>,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()>;

	/// If the entity has a component of type `T`, push the token stream,
	/// using its span if it exists.
	fn try_push_custom<T: Component + IntoCustomTokens>(
		&self,
		spans: &NonSendAssets<Span>,
		items: &mut Vec<TokenStream>,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<()> {
		if let Some(tokens) = self.maybe_spanned_custom(spans, entity, query)? {
			items.push(tokens);
		}
		Ok(())
	}

	/// create a token stream for an [`IntoCustomTokens`] item which may be spanned
	fn maybe_spanned_custom<T: Component + IntoCustomTokens>(
		&self,
		spans: &NonSendAssets<Span>,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<Option<TokenStream>> {
		if let Ok((item, span)) = query.get(entity) {
			if let Some(span) = span {
				let span = *spans.get(span)?;
				let item = item.into_custom_token_stream();
				Some(quote::quote_spanned! { span =>
					#item
				})
			} else {
				Some(item.into_custom_token_stream())
			}
		} else {
			None
		}
		.xok()
	}
	/// create a token stream for a [`NonSendHandle<syn::Expr>`] expression which may be spanned
	fn maybe_spanned_expr<
		T: Component + std::ops::Deref<Target = NonSendHandle<syn::Expr>>,
	>(
		&self,
		exprs_map: &NonSendAssets<syn::Expr>,
		spans: &NonSendAssets<Span>,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<Option<TokenStream>> {
		if let Ok((item, span)) = query.get(entity) {
			let item = exprs_map.get(item.deref())?;
			if let Some(span) = span {
				let span = *spans.get(span)?;
				let item = item.into_token_stream();
				Some(quote::quote_spanned! { span =>
					#item
				})
			} else {
				Some(item.into_token_stream())
			}
		} else {
			None
		}
		.xok()
	}
}

pub type MaybeSpannedQuery<'w, 's, T> = Query<
	'w,
	's,
	(&'static T, Option<&'static ItemOf<T, NonSendHandle<Span>>>),
>;

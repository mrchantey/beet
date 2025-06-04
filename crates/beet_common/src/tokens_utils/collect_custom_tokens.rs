use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use send_wrapper::SendWrapper;
use sweet::prelude::PipelineTarget;


/// A trait that can be implemented for types that implement [`SystemParam`]
pub trait CollectCustomTokens {
	/// For each [`Component`] in this type, push the token stream
	/// for that component if it is present.
	fn try_push_components(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()>;

	/// If the entity has a component of type `T`, push the token stream,
	/// using its span if it exists.
	fn try_push_custom<T: Component + IntoCustomTokens>(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<()> {
		if let Some(tokens) = self.maybe_spanned_custom(entity, query)? {
			items.push(tokens);
		}
		Ok(())
	}

	/// create a token stream for an [`IntoCustomTokens`] item which may be spanned
	fn maybe_spanned_custom<T: Component + IntoCustomTokens>(
		&self,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<Option<TokenStream>> {
		if let Ok((item, span)) = query.get(entity) {
			if let Some(span) = span {
				let item = item.into_custom_token_stream();
				Some(quote::quote_spanned! { ***span =>
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
}

pub type MaybeSpannedQuery<'w, 's, T> =
	Query<'w, 's, (&'static T, Option<&'static ItemOf<T, SendWrapper<Span>>>)>;

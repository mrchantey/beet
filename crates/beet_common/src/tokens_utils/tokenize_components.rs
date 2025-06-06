use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use send_wrapper::SendWrapper;
use sweet::prelude::PipelineTarget;

/// Query for an item that may have an associated ItemOf Span, allowing
/// the tokenizer to span the output token stream in
/// [`TokenizeComponents::tokenize_component`].
pub type MaybeSpannedQuery<'w, 's, T> =
	Query<'w, 's, (&'static T, Option<&'static ItemOf<T, SendWrapper<Span>>>)>;




/// A trait that can be implemented for types that implement [`SystemParam`]
pub trait TokenizeComponents {
	/// For each [`Component`] in this type, push the token stream
	/// for that component if it is present.
	fn tokenize_components(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()>;

	/// If the entity has a component of type `T`, push the token stream,
	/// using its span if it exists.
	fn tokenize_component<T: Component + IntoCustomTokens>(
		items: &mut Vec<TokenStream>,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<()> {
		if let Some(tokens) = tokenize_maybe_spanned_query(entity, query)? {
			items.push(tokens);
		}
		Ok(())
	}
}
/// create a token stream for an [`IntoCustomTokens`] item which may be spanned
fn tokenize_maybe_spanned_query<T: Component + IntoCustomTokens>(
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

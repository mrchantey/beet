use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;


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
	fn try_push<T: Component + IntoCustomTokens>(
		&self,
		spans: &NonSendAssets<Span>,
		items: &mut Vec<TokenStream>,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<()> {
		if let Ok((item, span)) = query.get(entity) {
			let tokens = if let Some(span) = span {
				let span = *spans.get(span)?;
				let item = item.into_custom_token_stream();
				quote::quote_spanned! { span =>
					#item
				}
			} else {
				item.into_custom_token_stream()
			};
			items.push(tokens);
		}
		Ok(())
	}
}

pub type MaybeSpannedQuery<'w, 's, T> =
	Query<'w, 's, (&'static T, Option<&'static NonSendHandle<Span>>)>;



/// Implement CollectCustomTokens for a struct with provided field names:
///
/// ## Example
/// The following example creates a struct `CollectRsxNodeTokens` and
/// implements the [`CollectCustomTokens`] trait for it.
/// ```rust
/// # use beet_common::prelude::*;
///
/// define_token_collector!(
///		CollectRsxNodeTokens,
///		node_tags: NodeTag,
///		fragments: FragmentNode,
///		texts: TextNode,
///		blocks: BlockNode,
///	);
///
///	```
#[macro_export]
macro_rules! define_token_collector {
		($name:ident, $($field:ident: $type:ty),* $(,)?) => {
		#[cfg(feature = "tokens")]
		#[derive(bevy::ecs::system::SystemParam)]
			pub struct $name<'w, 's> {
				$(
					$field: MaybeSpannedQuery<'w, 's, $type>,
				)*
			}

		#[cfg(feature = "tokens")]
		impl CollectCustomTokens for $name<'_, '_> {
			fn try_push_all(
				&self,
				spans: &NonSendAssets<proc_macro2::Span>,
				items: &mut Vec<proc_macro2::TokenStream>,
				entity: Entity,
			) -> Result<()> {
				$(
					self.try_push(spans, items, entity, &self.$field)?;
				)*
				Ok(())
			}
		}
	};
}

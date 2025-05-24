/// Implement CollectCustomTokens for a struct with provided field names:
///
/// ## Example
/// The following example creates a struct `CollectRsxNodeTokens` and
/// implements the [`CollectCustomTokens`] trait for it.
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_common::as_beet::*;
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
				items: &mut Vec<proc_macro2::TokenStream>,
				entity: Entity,
			) -> Result<()> {
				$(
					self.try_push_custom(items, entity, &self.$field)?;
				)*
				Ok(())
			}
		}
	};
}

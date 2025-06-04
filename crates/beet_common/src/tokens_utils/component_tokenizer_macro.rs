/// Implement ComponentTokenizer for a struct,
/// tokenizing each component defined in the fields.
///
/// ## Example
/// The following example creates a struct `CollectRsxNodeTokens` and
/// implements the [`ComponentTokenizer`] trait for it.
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_common::as_beet::*;
///
/// component_tokenizer!(
///		CollectRsxNodeTokens,
///		node_tags: NodeTag,
///		fragments: FragmentNode,
///		texts: TextNode,
///		blocks: BlockNode,
///	);
///
///	```
#[macro_export]
macro_rules! component_tokenizer {
		($name:ident, $($field:ident: $type:ty),* $(,)?) => {
		#[cfg(feature = "tokens")]
		#[derive(bevy::ecs::system::SystemParam)]
			pub struct $name<'w, 's> {
				$(
					$field: MaybeSpannedQuery<'w, 's, $type>,
				)*
			}

		#[cfg(feature = "tokens")]
		impl ComponentTokenizer for $name<'_, '_> {
			fn tokenize_components(
				&self,
				items: &mut Vec<proc_macro2::TokenStream>,
				entity: Entity,
			) -> Result<()> {
				$(
					Self::tokenize_component(items, entity, &self.$field)?;
				)*
				Ok(())
			}
		}
	};
}

/// Implement TokenizeComponents for a struct,
/// tokenizing each component defined in the fields.
///
/// ## Example
/// The following example creates a struct `TokenizeRsxNodes` and
/// implements the [`TokenizeComponents`] trait for it.
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_common::as_beet::*;
///
/// tokenize_components!(
///		TokenizeRsxNodes,
///		node_tags: NodeTag,
///		fragments: FragmentNode,
///		texts: TextNode,
///		blocks: BlockNode,
///	);
///
///	```
#[macro_export]
macro_rules! tokenize_components {
		($name:ident, $($field:ident: $type:ty),* $(,)?) => {
		#[cfg(feature = "tokens")]
		#[derive(bevy::ecs::system::SystemParam)]
			pub struct $name<'w, 's> {
				$(
					$field: MaybeSpannedQuery<'w, 's, $type>,
				)*
			}

		#[cfg(feature = "tokens")]
		impl TokenizeComponents for $name<'_, '_> {
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

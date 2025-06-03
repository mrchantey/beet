use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;



#[derive(SystemParam, Deref)]
pub struct TokenizeRelated<'w, 's, T: Component> {
	query: Query<'w, 's, &'static T>,
	// children: Query<'w, 's, &'static Children>,
}

pub trait TokenizeIdent {
	fn tokenize_ident() -> Ident;
}
impl TokenizeIdent for Children {
	fn tokenize_ident() -> Ident { syn::parse_quote!(Children) }
}

impl<'w, 's, T> TokenizeRelated<'w, 's, T>
where
	T: Component + RelationshipTarget + TokenizeIdent,
{
pub	fn try_push_all(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
		map_child: impl Fn(Entity) -> Result<TokenStream>,
	) -> Result<()> {
		if let Ok(children) = self.query.get(entity) {
			let children =
				children.iter().map(map_child).collect::<Result<Vec<_>>>()?;
			if !children.is_empty() {
				let ident = T::tokenize_ident();
				items.push(quote! { related!{#ident [#(#children),*]} });
			}
		};

		Ok(())
	}
}

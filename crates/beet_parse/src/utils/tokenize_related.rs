use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// For a given [`RelationshipTarget`] component,
#[derive(SystemParam, Deref)]
pub struct TokenizeRelated<'w, 's, T: Component> {
	query: Query<'w, 's, &'static T>,
}

impl<'w, 's, T> TokenizeRelated<'w, 's, T>
where
	T: Component + RelationshipTarget + TypePath,
{
	pub fn try_push_related(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
		map_child: impl Fn(Entity) -> Result<TokenStream>,
	) -> Result<()> {
		if let Ok(children) = self.query.get(entity) {
			let children =
				children.iter().map(map_child).collect::<Result<Vec<_>>>()?;
			if !children.is_empty() {
				let ident = type_path_to_ident::<T>()?;
				items.push(quote! { related!{#ident [#(#children),*]} });
			}
		};

		Ok(())
	}
}

fn type_path_to_ident<T: TypePath>() -> Result<Ident> {
	let ident = T::type_ident().ok_or_else(|| {
		anyhow::anyhow!(
			"Failed to get type identifier for component: {}",
			std::any::type_name::<T>()
		)
	})?;
	let ident: Ident = syn::parse_str(ident).map_err(|_| {
		anyhow::anyhow!(
			"Failed to parse type identifier for component: {}",
			std::any::type_name::<T>()
		)
	})?;

	Ok(ident)
}

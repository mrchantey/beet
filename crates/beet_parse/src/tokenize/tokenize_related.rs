use crate::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// If the entity has this [`RelationshipTarget`], then map each
/// child with `map_child` and return a `related!` [`TokenStream`]
pub fn tokenize_related<T: Component + RelationshipTarget + TypePath>(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
	map_child: impl Fn(&World, Entity) -> Result<TokenStream>,
) -> Result<()> {
	let entity = world.entity(entity);
	let Some(related) = entity.get::<T>().map(|c| c.iter().collect::<Vec<_>>())
	else {
		return Ok(());
	};
	if related.is_empty() {
		return Ok(());
	}
	let related = related
		.into_iter()
		.map(|child| map_child(world, child))
		.collect::<Result<Vec<_>>>()?;
	let ident = type_path_to_ident::<T>()?;
	items.push(unbounded_related(&ident, related));
	Ok(())
}



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

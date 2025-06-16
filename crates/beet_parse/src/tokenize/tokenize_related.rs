use beet_common::prelude::TokenizeSelf;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// bundle impl limit
pub const MAX_BUNDLE_TUPLE: usize = 12;

/// 1. If the entity has no children, return `()`
/// 2. If the entity has a single child, map that child with `map_child`.
pub fn flatten_fragment(
	world: &World,
	entity: Entity,
	map_child: impl Fn(&World, Entity) -> Result<TokenStream>,
) -> Result<TokenStream> {
	let Some(children) = world
		.entity(entity)
		.get::<Children>()
		.map(|c| c.iter().collect::<Vec<_>>())
	else {
		return Ok(().self_token_stream());
	};
	if children.len() == 1 {
		// a single child, return that
		map_child(world, children[0])
	} else {
		// multiple children, wrap in fragment
		let children = children
			.into_iter()
			.map(|child| map_child(world, child))
			.collect::<Result<Vec<_>>>()?;
		Ok(quote! { (
			FragmentNode,
			children![#(#children),*])
			// related!{Children,[#(#children),*]})
		})
	}
}

/// If the entity has this [`RelationshipTarget`], then map each
/// child with `map_child` and return a `related!` [`TokenStream`]
pub fn tokenize_related<T: Component + RelationshipTarget + TypePath>(
	world: &World,
	items: &mut Vec<TokenStream>,
	entity: Entity,
	map_child: impl Fn(&World, Entity) -> Result<TokenStream>,
) -> Result<()>
where
	T::Relationship: TypePath,
{
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
	let child_ident = type_path_to_ident::<T::Relationship>()?;

	if related.len() <= MAX_BUNDLE_TUPLE {
		items.push(quote! { related!{#ident [#(#related),*]} });
	} else {
		// must be send+sync
		items.push(quote! {#ident::spawn(
			bevy::ecs::spawn::SpawnWith(
				|parent: &mut bevy::ecs::relationship::RelatedSpawner<#child_ident>| {
					#(parent.spawn(#related);)*
				},
			)
		)});
	}
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

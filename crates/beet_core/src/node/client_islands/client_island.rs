use crate::prelude::*;
use bevy::ecs::component::Immutable;
use bevy::ecs::component::StorageType;
use bevy::prelude::*;




#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClientIsland {
	pub template: TemplateSerde,
	/// Whether to create html elements instead of hydrating them,
	/// if the template is [`ClientOnlyDirective`], this will be true.
	pub mount_to_dom: bool,
	pub dom_idx: DomIdx, // pub route: RouteInfo,
}


#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SpawnClientIsland<T, U = T>
where
	T: 'static + Send + Sync + IntoTemplateBundle<U>,
	U: 'static + Send + Sync,
{
	value: T,
	#[reflect(ignore)]
	phantom: std::marker::PhantomData<U>,
}


impl<T, U> Component for SpawnClientIsland<T, U>
where
	T: 'static + Send + Sync + IntoTemplateBundle<U>,
	U: 'static + Send + Sync,
{
	const STORAGE_TYPE: StorageType = StorageType::Table;
	type Mutability = Immutable;


	fn on_add() -> Option<bevy::ecs::component::ComponentHook> {
		Some(|mut world, cx| {
			let entity = cx.entity;
			world.commands().queue(move |world: &mut World| -> Result {
				let this = world.entity_mut(entity).take::<Self>().ok_or_else(
					|| {
						let name = std::any::type_name::<Self>();
						bevyhow!(
							"SpawnClientIsland<{}> component not found on entity: {:?}",
							name,
							entity
						)
					},
				)?;
				world
					.entity_mut(entity)
					.insert(this.value.into_node_bundle());
				Ok(())
			});
		})
	}
}

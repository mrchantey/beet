use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;


pub struct ReplicateComponentPlugin<T: ReplicateComponent> {
	phantom: PhantomData<T>,
}


impl<T: ReplicateComponent> Plugin for ReplicateComponentPlugin<T> {
	fn build(&self, app: &mut App) {
		app.init_resource::<Registrations>();
		let mut registrations = app.world_mut().resource_mut::<Registrations>();
		T::register(&mut registrations);
	}
}


pub struct ComponentFns {
	pub type_id: std::any::TypeId,
	pub insert: fn(&mut EntityCommands, payload: &[u8]) -> bincode::Result<()>,
	pub change: fn(&mut EntityCommands, payload: &[u8]) -> bincode::Result<()>,
	pub remove: fn(&mut EntityCommands),
}

fn handle_change<T: Component + Serialize>(
	registrations: Res<Registrations>,
	mut outgoing: EventWriter<MessageOutgoing>,
	query: Query<(Entity, &T), (Changed<T>, With<Replicate>)>,
) {
	for (entity, component) in query.iter() {
		let Some(bytes) =
			bincode::serialize(component).ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};

		outgoing.send(
			Message::Change {
				entity,
				reg_id: registrations.registration_id::<T>(),
				bytes,
			}
			.into(),
		);
	}
}

fn handle_insert<T: Component + Serialize>(
	registrations: Res<Registrations>,
	mut outgoing: EventWriter<MessageOutgoing>,
	query: Query<(Entity, &T), (Added<T>, With<Replicate>)>,
) {
	for (entity, component) in query.iter() {
		let Some(bytes) =
			bincode::serialize(component).ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};
		outgoing.send(
			Message::Change {
				entity,
				reg_id: registrations.registration_id::<T>(),
				bytes,
			}
			.into(),
		);
	}
}



/// Ignores despawned entities
fn handle_remove<T: Component>(
	registrations: Res<Registrations>,
	mut outgoing: EventWriter<MessageOutgoing>,
	mut removed: RemovedComponents<T>,
	query: Query<(), With<Replicate>>,
) {
	for removed in removed.read() {
		if query.contains(removed) {
			outgoing.send(
				Message::Remove {
					entity: removed,
					reg_id: registrations.registration_id::<T>(),
				}
				.into(),
			);
		}
	}
}


pub trait ReplicateComponent: 'static + Send + Sync {
	fn register(registrations: &mut Registrations);
	fn update_systems() -> SystemConfigs;
}

impl<T: Send + Sync + 'static + Component + Serialize + DeserializeOwned>
	ReplicateComponent for T
{
	fn register(registrations: &mut Registrations) {
		registrations.register_component(ComponentFns {
			type_id: std::any::TypeId::of::<T>(),
			insert: |commands, payload| {
				let component: T = bincode::deserialize(payload)?;
				commands.insert(component);
				Ok(())
			},
			change: |commands, payload| {
				let component: T = bincode::deserialize(payload)?;
				commands.insert(component);
				Ok(())
			},
			remove: |commands| {
				commands.remove::<T>();
			},
		});
	}

	fn update_systems() -> SystemConfigs {
		(handle_insert::<T>, handle_change::<T>, handle_remove::<T>)
			.into_configs()
	}
}

// macro_rules! impl_replicate_component_tuples {
// 	($($param: ident),*) => {
// 			impl<$($param),*> ReplicateComponent for ($($param,)*)
// 			where
// 					$($param: Send + Sync + 'static + Component + Serialize + DeserializeOwned),*
// 			{
// 					#[allow(non_snake_case, unused_variables)]
// 					#[track_caller]
// 					fn register(registrations:&mut Registrations) {
// 							$($param::register(registrations);)*
// 					}
// 					fn update_systems()-> SystemConfigs {
// 							($($param::update_systems(),)*).into_configs()
// 					}
// 			}
// 	}
// }
// all_tuples!(impl_replicate_component_tuples, 1, 15, P);

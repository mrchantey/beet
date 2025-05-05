use crate::prelude::*;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;



pub struct BevyEventRegistry;

impl Plugin for BevyEventRegistry {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, interaction_system);
	}
}



#[derive(Debug, Clone, Event)]
pub struct ClickEvt;
#[derive(Debug, Clone, Event)]
pub struct HoverStart;
#[derive(Debug, Clone, Event)]
pub struct HoverEnd;

/// To be used with the [`BevyEventRegistry`] to trigger events
fn interaction_system(
	query: Populated<(Entity, &Interaction), Changed<Interaction>>,
	mut commands: Commands,
) {
	for (entity, interaction) in query.iter() {
		match interaction {
			Interaction::Pressed => {
				commands.entity(entity).trigger(ClickEvt);
			}
			Interaction::Hovered => {
				commands.entity(entity).trigger(HoverStart);
			}
			Interaction::None => {
				commands.entity(entity).trigger(HoverEnd);
			}
		}
	}
}

impl BevyEventRegistry {
	/// Any observer is accepted here and will be attached to the entity
	pub fn register_on<E, B, M>(
		_key: &str,
		loc: TreeLocation,
		observer: impl IntoObserverSystem<E, B, M>,
	) where
		E: bevy::prelude::Event,
		B: Bundle,
	{
		BevyRuntime::with_mut(move |app| {
			let mut query = app.world_mut().query::<(Entity, &TreeIdx)>();
			let entity = TreeIdx::find(query.iter(app.world()), loc)
				.expect(&expect_rsx_element::to_be_at_location(&loc));
			app.world_mut().entity_mut(entity).observe(observer);
			app.world_mut().flush();
		});

		// Self::register(key, loc, value);
	}
	/// Called on Event Registration, which is after entities have
	/// been mounted.
	pub fn register_onclick(
		_key: &str,
		loc: TreeLocation,
		value: impl 'static + Send + Sync + Fn(Trigger<ClickEvt>),
	) {
		Self::register_on(_key, loc, value);
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn registers() {
		BevyRuntime::reset();
		BevyRuntime::with_mut(|app| {
			app.add_plugins(BevyEventRegistry);
			let bucket = mock_bucket();
			let bucket2 = bucket.clone();
			let world = app.world_mut();
			let entity = world
				.spawn_empty()
				.observe(move |_: Trigger<ClickEvt>| {
					bucket2.call(());
				})
				.id();
			world.flush();
			world.entity_mut(entity).trigger(ClickEvt);
			expect(&bucket).to_have_been_called_times(1);
		});
	}

	#[test]
	fn macro_works() {
		BevyRuntime::reset();
		BevyRuntime::with_mut(|app| {
			app.add_plugins(BevyEventRegistry);
		});

		let bucket = mock_bucket();
		let bucket2 = bucket.clone();

		let rsx = rsx! {
			<entity
				runtime:bevy
				onclick=move |_| {
					bucket2.call(());
				}
			/>
		};
		let entity = RsxToBevy::spawn(rsx).unwrap()[0];

		BevyRuntime::with_mut(|app| {
			app.world_mut().entity_mut(entity).trigger(ClickEvt);
			app.world_mut().flush();
		});
		expect(&bucket).to_have_been_called_times(1);
	}
}

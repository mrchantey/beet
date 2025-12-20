use crate::prelude::*;
use bevy::ecs::change_detection::Tick;
use bevy::ecs::system::ReadOnlySystemParam;
use bevy::ecs::system::SystemMeta;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::SystemParamValidationError;
use bevy::ecs::world::DeferredWorld;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;


/// A [`SystemParam`] that wraps another parameter and causes its system to skip instead of failing when the parameter is invalid.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// // This system will fail if `SomeResource` is not present.
/// fn fails_on_missing_resource(res: Res<SomeResource>) {}
///
/// // This system will skip without error if `SomeResource` is not present.
/// fn skips_on_missing_resource(res: When<Res<SomeResource>>) {
///     // The inner parameter is available using `Deref`
///     let some_resource: &SomeResource = &res;
/// }
/// ```
#[derive(Debug)]
pub struct When<T>(pub T);

impl<T> When<T> {
	/// Returns the inner `T`.
	pub fn into_inner(self) -> T { self.0 }
}

impl<T> std::ops::Deref for When<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl<T> std::ops::DerefMut for When<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: SystemParam> SystemParam for When<T> {
	type State = T::State;

	type Item<'world, 'state> = When<T::Item<'world, 'state>>;

	fn init_state(world: &mut World) -> Self::State { T::init_state(world) }
	fn init_access(
		state: &Self::State,
		system_meta: &mut SystemMeta,
		component_access_set: &mut bevy::ecs::query::FilteredAccessSet,
		world: &mut World,
	) {
		T::init_access(state, system_meta, component_access_set, world);
	}

	#[inline]
	unsafe fn validate_param(
		state: &mut Self::State,
		system_meta: &SystemMeta,
		world: UnsafeWorldCell,
	) -> Result<(), SystemParamValidationError> {
		unsafe {
			T::validate_param(state, system_meta, world).map_err(|mut e| {
				e.skipped = true;
				e
			})
		}
	}

	#[inline]
	unsafe fn get_param<'world, 'state>(
		state: &'state mut Self::State,
		system_meta: &SystemMeta,
		world: UnsafeWorldCell<'world>,
		change_tick: Tick,
	) -> Self::Item<'world, 'state> {
		When(unsafe { T::get_param(state, system_meta, world, change_tick) })
	}

	fn apply(
		state: &mut Self::State,
		system_meta: &SystemMeta,
		world: &mut World,
	) {
		T::apply(state, system_meta, world);
	}

	fn queue(
		state: &mut Self::State,
		system_meta: &SystemMeta,
		world: DeferredWorld,
	) {
		T::queue(state, system_meta, world);
	}
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: ReadOnlySystemParam> ReadOnlySystemParam for When<T> {}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Default, Resource)]
	struct Foo;

	#[test]
	#[ignore = "noisy"]
	#[should_panic]
	fn default() { App::new().add_systems(Update, |_res: Res<Foo>| {}).run(); }

	#[test]
	#[ignore = "noisy"]
	#[should_panic]
	fn panics() {
		App::new()
			.init_resource::<Foo>()
			.add_systems(Update, |_res: When<Res<Foo>>| {
				panic!("this will be reached")
			})
			.run();
	}
	#[test]
	fn doesnt_panic() {
		App::new()
			.add_systems(Update, |_res: When<Res<Foo>>| {
				panic!("this wont be reached")
			})
			.run();
	}
}

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


/// A system param for extracting types from request params,
/// and caching them by inserting as components alongside the request.
/// ## Note
///
/// This query should not be used in route handlers, as it accepts
/// an agent entity, not an action entity, instead see [`RouteParamQuery`]
#[derive(SystemParam)]
pub struct ParamQuery<'w, 's, T: Component> {
	pub commands: Commands<'w, 's>,
	pub agents: Query<'w, 's, (&'static RequestMeta, Option<&'static T>)>,
}

impl<T: Clone + Component> ParamQuery<'_, '_, T> {
	/// Attempt to extract the param from the request, inserting it into the
	/// request entity if it is missing.
	pub fn get(&mut self, agent: Entity) -> Result<T>
	where
		T: Sized + Clone + FromReflect + Typed + Component,
	{
		self.get_custom(agent, |request| request.params().parse_reflect::<T>())
	}
	/// Attempt to extract the param from the request using a custom function
	pub fn get_custom<F>(&mut self, agent: Entity, func: F) -> Result<T>
	where
		F: FnOnce(&RequestMeta) -> Result<T>,
	{
		let (request, params) = self.agents.get(agent)?;
		match params {
			Some(params) => Ok(params.clone()),
			None => {
				let params = func(request)?;
				self.commands.entity(agent).insert(params.clone());
				Ok(params)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn works() {
		#[derive(Reflect, Component, Clone)]
		struct Foo {
			foo: bool,
		}

		let mut world = World::new();
		let entity = world.spawn(Request::from_cli_str("--foo").unwrap()).id();
		world
			.run_system_once(move |mut foo: ParamQuery<Foo>| {
				foo.get(entity).unwrap()
			})
			.unwrap()
			.foo
			.xpect_true();
		// assigns component
		world.entity(entity).get::<Foo>().unwrap().foo.xpect_true();
	}
}

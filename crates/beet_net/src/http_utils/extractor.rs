use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


// TODO so similar to FromRequestMeta
pub trait RequestMetaExtractor:
	Sized + Clone + FromReflect + Typed + Component
{
	fn extract(request: &RequestMeta) -> Result<Self>;
}


/// A system param for extracting types from request params,
/// and caching them by inserting as components alongside the request.
#[derive(SystemParam)]
pub struct Extractor<'w, 's, T: Component> {
	pub commands: Commands<'w, 's>,
	pub requests: Query<'w, 's, (&'static RequestMeta, Option<&'static T>)>,
}


impl<T: RequestMetaExtractor> Extractor<'_, '_, T> {
	/// Attempt to extract the param from the request, inserting it into the
	/// request entity if it is missing.
	pub fn get(&mut self, exchange_entity: Entity) -> Result<T> {
		let (request, extractor) = self.requests.get(exchange_entity)?;
		if let Some(extractor) = extractor {
			return Ok(extractor.clone());
		} else {
			let value = T::extract(request)?;
			self.commands.entity(exchange_entity).insert(value.clone());
			Ok(value)
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		#[derive(Reflect, Component, Clone)]
		struct Foo {
			foo: bool,
		}

		impl RequestMetaExtractor for Foo {
			fn extract(request: &RequestMeta) -> Result<Self> {
				request.params().parse_reflect()
			}
		}

		let mut world = World::new();
		let entity = world.spawn(Request::from_cli_str("--foo").unwrap()).id();
		world
			.run_system_once(move |mut foo: Extractor<Foo>| {
				foo.get(entity).unwrap()
			})
			.unwrap()
			.foo
			.xpect_true();
		// assigns component
		world.entity(entity).get::<Foo>().unwrap().foo.xpect_true();
	}
}

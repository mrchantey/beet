use crate::prelude::*;
use beet_core::prelude::*;


/// A tool that increments a specified field when triggered, returning the new value.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Increment {
	/// Path to the field to increment.
	pub field: FieldRef,
}


impl Tool for Increment {
	type In = ();
	type Out = i64;

	fn call(
		entity: AsyncEntity,
		_input: Self::In,
	) -> impl Future<Output = Result<Self::Out>> {
		async move {
			let increment = entity.get_cloned::<Increment>().await?;
			


			// Placeholder implementation
			Ok(42)
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn increment() {
		let mut world = World::new();
		world.spawn(Increment::default());
	}
}

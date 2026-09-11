//! [`RelationMeta`]: relation semantics as registered type data.
use crate::prelude::*;
use bevy_reflect::GetTypeRegistration;
use bevy_reflect::TypeRegistry;

/// The semantics of a relationship component that bevy enforces in the live
/// world but the registry cannot otherwise answer, registered as type data
/// beside the component's reflect registration.
///
/// A document edit happens before the world sees it, so the facts a write is
/// checked against and an entity picker filters by live here: a `ChildOf` may
/// never lead back to its own entity, which the world would only discover by
/// panicking. A relation registering no meta stays free to cycle and
/// self-reference.
///
/// Consumed by [`SceneEntities::assert_acyclic`], whose exhaustive destructure is
/// the tripwire: a new fact fails to compile there until the check says what it
/// means.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct RelationMeta {
	/// The relation may never lead back to its source: a target that is the
	/// source, or that reaches it through the same relation, is rejected.
	pub acyclic: bool,
}

impl RelationMeta {
	/// A relation that may never cycle, ie a `ChildOf`.
	pub fn acyclic() -> Self { Self { acyclic: true } }

	/// Register these semantics beside `T`'s reflect registration, registering
	/// `T` first if `types` does not hold it.
	pub fn register<T: GetTypeRegistration>(self, types: &mut TypeRegistry) {
		types.register::<T>();
		types
			.get_mut(core::any::TypeId::of::<T>())
			.expect("registered above")
			.insert(self);
	}

	/// The semantics registered for the type at `type_path`, if it declared
	/// any.
	pub fn of<'a>(
		types: &'a TypeRegistry,
		type_path: &str,
	) -> Option<&'a Self> {
		types.get_with_type_path(type_path)?.data::<Self>()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy_reflect::TypeRegistry;

	#[crate::test]
	fn registers_beside_the_type() {
		let mut types = TypeRegistry::default();
		RelationMeta::acyclic().register::<ChildOf>(&mut types);
		RelationMeta::of(&types, ChildOf::type_path())
			.unwrap()
			.xpect_eq(RelationMeta::acyclic());
		// an unmarked relation answers nothing, and stays free to cycle
		types.register::<Name>();
		RelationMeta::of(&types, Name::type_path()).xpect_none();
	}

	/// The hierarchy is marked by the document plugin, so every app rejects a
	/// cyclic `ChildOf` edit without opting in.
	#[crate::test]
	fn the_hierarchy_is_marked_by_default() {
		let world = DocumentPlugin::world();
		let types = world.resource::<AppTypeRegistry>().read();
		RelationMeta::of(&types, ChildOf::type_path())
			.unwrap()
			.acyclic
			.xpect_true();
	}
}

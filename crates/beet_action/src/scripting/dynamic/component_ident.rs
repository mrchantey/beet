//! Resolving a component identifier, as written in a scene or a script, to a
//! live component.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::reflect::TypeRegistry;
use core::any::TypeId;

/// A component identifier resolved against a [`World`].
///
/// One identifier space covers both kinds of component: a registered rust type
/// is named by its type path (short or full), a runtime-minted one by the name
/// its [`DynamicComponent`] declaration gave it. Everything downstream, the
/// reads, the writes and the exposure checks, addresses components through this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentIdent {
	/// The identifier as it appears on the wire: a registered component's full
	/// type path, or a dynamic component's declared name.
	pub path: SmolStr,
	/// The short form a human writes: a registered component's short type path,
	/// or, for a dynamic component, its declared name again.
	///
	/// Carried alongside [`path`](Self::path) because a [`ScriptExposure`] is
	/// written either way and must match either way.
	pub short: SmolStr,
	/// The live component.
	pub id: ComponentId,
	/// The reflected type, absent for a dynamic component, which has none.
	pub type_id: Option<TypeId>,
}

impl ComponentIdent {
	/// Resolve `ident` against the world's dynamic vocabulary, then its type
	/// registry.
	///
	/// A registered component is registered with the world on demand, so an
	/// identifier resolves whether or not any entity carries it yet.
	///
	/// # Errors
	/// Errors naming the identifier, and any near misses, when nothing matches.
	pub fn resolve(world: &mut World, ident: &str) -> Result<Self> {
		if let Some(id) = DynamicComponents::get(world, ident) {
			return Self {
				path: ident.into(),
				short: ident.into(),
				id,
				type_id: None,
			}
			.xok();
		}
		// the registry lock is released before the world is borrowed mutably to
		// register the component, so the two never overlap.
		let registry = world.resource::<AppTypeRegistry>().clone();
		let resolved = {
			let registry = registry.read();
			match reflect_ext::registration_by_name(&registry, ident) {
				Some(registration) => Some((
					SmolStr::new(registration.type_info().type_path()),
					SmolStr::new(
						registration.type_info().type_path_table().short_path(),
					),
					registration.type_id(),
					registration
						.data::<ReflectComponent>()
						.cloned()
						.ok_or_else(|| {
							bevyhow!(
								"`{ident}` is a registered type but not a component: \
it is missing `#[reflect(Component)]`"
							)
						})?,
				)),
				None => None,
			}
		};
		let Some((path, short, type_id, reflect_component)) = resolved else {
			return Err(Self::unknown(&registry.read(), ident));
		};
		Self {
			path,
			short,
			id: reflect_component.register_component(world),
			type_id: Some(type_id),
		}
		.xok()
	}

	/// Whether this identifier names a runtime-minted component rather than a
	/// registered rust type.
	pub fn is_dynamic(&self) -> bool { self.type_id.is_none() }

	/// The error for an identifier nothing matched, naming the near misses.
	fn unknown(registry: &TypeRegistry, ident: &str) -> BevyError {
		let near = registry
			.iter()
			.map(|registration| {
				registration.type_info().type_path_table().short_path()
			})
			.filter(|short| {
				short.eq_ignore_ascii_case(ident)
					|| short.contains(ident)
					|| ident.contains(*short)
			})
			.take(4)
			.collect::<Vec<_>>();
		match near.is_empty() {
			true => bevyhow!(
				"unknown component `{ident}`. Registered components are named by \
type path; a runtime component must be declared first, ie \
`<DynamicComponent name=\"{ident}\"/>`"
			),
			false => bevyhow!(
				"unknown component `{ident}`. Did you mean one of {near:?}?"
			),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn resolves_a_short_path() {
		let mut world = test_world();
		ComponentIdent::resolve(&mut world, "Name")
			.unwrap()
			.path
			.xpect_eq(SmolStr::new("bevy_ecs::name::Name"));
	}

	#[beet_core::test]
	fn resolves_a_full_path() {
		let mut world = test_world();
		ComponentIdent::resolve(&mut world, "bevy_ecs::name::Name")
			.unwrap()
			.is_dynamic()
			.xpect_false();
	}

	#[beet_core::test]
	fn resolves_a_dynamic_component() {
		let mut world = test_world();
		DynamicComponents::register(
			&mut world,
			"guestbook.Flagged",
			ValueSchema::Any,
		)
		.unwrap();
		ComponentIdent::resolve(&mut world, "guestbook.Flagged")
			.unwrap()
			.is_dynamic()
			.xpect_true();
	}

	#[beet_core::test]
	fn an_unknown_identifier_names_itself() {
		let mut world = test_world();
		ComponentIdent::resolve(&mut world, "Nonesuch")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown component `Nonesuch`");
	}

	#[beet_core::test]
	fn a_near_miss_is_suggested() {
		let mut world = test_world();
		ComponentIdent::resolve(&mut world, "Nam")
			.unwrap_err()
			.to_string()
			.xpect_contains("Did you mean");
	}
}

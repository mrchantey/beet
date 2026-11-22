//! Component types minted at runtime, with no rust definition behind them.
use beet_core::prelude::*;

/// Declares a component type that has no rust definition, minted when this
/// declaration is spawned.
///
/// ```ignore
/// <DynamicComponent name="guestbook.Visits" schema="u64"/>
/// ```
///
/// Minting vocabulary without recompiling is the point, so this is a component a
/// scene keeps rather than a template that expands away: a scene that declares
/// `guestbook.Visits` still declares it after a serde round trip.
///
/// The value is a [`Value`], the same thing a document field holds, so a runtime
/// component carries whatever a document can: a count, a flag, a string, a list,
/// a map. That is enough to give a scene words the engine never shipped.
///
/// The [`schema`](Self::schema) is what those words *mean*. Left at
/// [`Any`](ValueSchema::Any) a runtime component accepts anything, which is the
/// undeclared behaviour; name a schema and every write is validated against it
/// before it reaches the component's storage, including a write from a
/// sandboxed script, which sees a rejection as the same catchable error an
/// config refusal is.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(DynamicComponent::mint))]
pub struct DynamicComponent {
	/// The name the component is addressed by, in a scene and in a script.
	pub name: SmolStr,
	/// What this component accepts. [`Any`](ValueSchema::Any) accepts anything.
	pub schema: ValueSchema,
}

/// An undeclared schema is [`Any`](ValueSchema::Any), not the [`Null`] a bare
/// `ValueSchema` defaults to: a runtime component with nothing said about it
/// accepts anything, which is what it did before schemas existed.
///
/// [`Null`]: ValueSchema::Null
impl Default for DynamicComponent {
	fn default() -> Self {
		Self {
			name: SmolStr::default(),
			schema: ValueSchema::Any,
		}
	}
}

impl DynamicComponent {
	/// Declare a runtime component named `name`, accepting anything.
	pub fn new(name: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			..default()
		}
	}

	/// Narrow what this component accepts.
	pub fn with_schema(mut self, schema: ValueSchema) -> Self {
		self.schema = schema;
		self
	}

	/// The hook body: register the declared name once the command queue flushes.
	///
	/// Registration is structural, which a [`DeferredWorld`](bevy::ecs::world::DeferredWorld)
	/// cannot do, so it is queued rather than performed in the hook. A
	/// conflicting redeclaration therefore has no caller to return to, and goes
	/// out through the world's error handler.
	fn mint(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let declaration = self.clone();
		move |entity: &mut EntityCommands| {
			entity.commands().queue(move |world: &mut World| {
				if let Err(err) = DynamicComponents::register(
					world,
					&declaration.name,
					declaration.schema,
				) {
					world.handle_command_error::<DynamicComponent>(err);
				}
			});
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn registering_twice_yields_one_component() {
		let mut world = World::new();
		let first = DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::Any,
		)
		.unwrap();
		let second = DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::Any,
		)
		.unwrap();
		first.xpect_eq(second);
	}

	/// The name is the identity, so two declarations disagreeing about what it
	/// means is a conflict rather than a last-writer-wins.
	#[beet_core::test]
	fn a_conflicting_redeclaration_errors() {
		let mut world = World::new();
		DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::Any,
		)
		.unwrap();
		DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::U64(default()),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("`game.Health` is already declared as Any");
	}

	#[beet_core::test]
	fn distinct_names_are_distinct_components() {
		let mut world = World::new();
		let health = DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::Any,
		)
		.unwrap();
		let mana = DynamicComponents::register(
			&mut world,
			"game.Mana",
			ValueSchema::Any,
		)
		.unwrap();
		(health == mana).xpect_false();
	}

	#[beet_core::test]
	fn a_declaration_mints_its_component() {
		let mut world = World::new();
		world.spawn(DynamicComponent::new("game.Health"));
		world.flush();
		DynamicComponents::get(&world, "game.Health")
			.is_some()
			.xpect_true();
	}

	/// A declaration carries its schema through the hook, so a scene declaring
	/// what a word means is what the write path then enforces.
	#[beet_core::test]
	fn a_declaration_keeps_its_schema() {
		let mut world = World::new();
		world.spawn(
			DynamicComponent::new("game.Health")
				.with_schema(ValueSchema::U64(default())),
		);
		world.flush();
		let id = DynamicComponents::get(&world, "game.Health").unwrap();
		DynamicComponents::schema_of(&world, id)
			.cloned()
			.unwrap()
			.xpect_eq(ValueSchema::U64(default()));
	}

	/// A runtime component is saved under its name and loaded back once the
	/// name is minted, whichever comes first: here the declaration sits after
	/// the entity using it, so the value waits held until it runs.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn a_runtime_component_round_trips_a_scene() {
		// a world saving the hierarchy and the declaration
		let scene_world = || {
			let world = test_world();
			world
				.resource::<AppTypeRegistry>()
				.write()
				.register::<(ChildOf, DynamicComponent)>();
			world
		};
		let mut world = scene_world();
		DynamicComponents::register(&mut world, "game.Loot", ValueSchema::Any)
			.unwrap();
		let root = world.spawn_empty().id();
		let looted = world.spawn(ChildOf(root)).id();
		WorldWrite::insert(
			&mut world,
			looted,
			"game.Loot",
			value!(["sword"]),
			&ScriptConfig::default(),
		)
		.unwrap();
		world.spawn((DynamicComponent::new("game.Loot"), ChildOf(root)));
		world.flush();
		let bytes = TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [root])
			.unwrap();
		bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains(r#""game.Loot":["sword"]"#);

		let mut loaded = scene_world();
		let root = TemplateLoader::new(&mut loaded).load(&bytes).unwrap()[0];
		loaded.flush();
		let looted = loaded.entity(root).get::<Children>().unwrap()[0];
		WorldRead::get(
			&mut loaded,
			looted,
			"game.Loot",
			&ScriptConfig::default(),
		)
		.unwrap()
		.unwrap()
		.xpect_eq(value!(["sword"]));
		loaded.get::<Unregistered>(looted).xpect_none();
		// and the reload re-saves the bytes it read
		TemplateSaver::new()
			.save_roots(&mut loaded, MediaType::Json, [root])
			.unwrap()
			.xpect_eq(bytes);
	}

	/// Insert a runtime component through the bridge and read it back, the round
	/// trip every script value takes.
	fn round_trip(value: Value) -> Value {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Loot", ValueSchema::Any)
			.unwrap();
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"game.Loot",
			value,
			&ScriptConfig::default(),
		)
		.unwrap();
		WorldRead::get(
			&mut world,
			entity,
			"game.Loot",
			&ScriptConfig::default(),
		)
		.unwrap()
		.unwrap()
	}

	/// A runtime component holds anything a document field can, not just a
	/// number: the heap-allocating cases are the ones the layout's drop fn has
	/// to get right.
	#[beet_core::test]
	fn round_trips_a_string() {
		round_trip(Value::from("sword")).xpect_eq(Value::from("sword"));
	}

	#[beet_core::test]
	fn round_trips_a_list() {
		round_trip(value!(["sword", "shield"]))
			.xpect_eq(value!(["sword", "shield"]));
	}

	#[beet_core::test]
	fn round_trips_a_map() {
		round_trip(value!({ "sword": 2u64 }))
			.xpect_eq(value!({ "sword": 2u64 }));
	}

	/// The wire is [`Value`], so bytes reach a component's storage as bytes
	/// rather than as the number list JSON would have made of them.
	#[beet_core::test]
	fn round_trips_bytes() {
		round_trip(Value::Bytes(vec![1, 2, 3]))
			.xpect_eq(Value::Bytes(vec![1, 2, 3]));
	}

	/// A type-less component has nothing for the reflect clone handler to work
	/// from, so the descriptor carries its own: without it an entity clone would
	/// silently drop the runtime vocabulary.
	#[beet_core::test]
	fn cloning_an_entity_clones_the_value() {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Loot", ValueSchema::Any)
			.unwrap();
		let source = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			source,
			"game.Loot",
			value!(["sword"]),
			&ScriptConfig::default(),
		)
		.unwrap();
		let target = world.entity_mut(source).clone_and_spawn();
		WorldRead::get(
			&mut world,
			target,
			"game.Loot",
			&ScriptConfig::default(),
		)
		.unwrap()
		.unwrap()
		.xpect_eq(value!(["sword"]));
	}
}

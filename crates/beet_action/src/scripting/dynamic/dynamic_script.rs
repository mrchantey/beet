//! The behaviour-tree leaf: a script that acts on the world through a `world`
//! API.
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value as JsonValue;

/// A script that acts on the world through a promise-shaped `world` API.
///
/// ```ignore
/// <DynamicScript script="
///   const [entry] = await world.entities('bevy_ecs::name::Name');
///   const name = await world.get(entry, 'bevy_ecs::name::Name');
///   const copy = await world.spawn({ 'bevy_ecs::name::Name': name + ' (copy)' });
///   await world.insert(copy, 'guestbook.Visits', 1);
/// "/>
/// ```
///
/// Every `world` method is served against the live world the moment it is
/// awaited: a read returns current state, a write lands immediately and in
/// order, a `spawn` resolves to a real entity id usable by the next line, and a
/// refused call rejects the promise where it was made, catchable in the script.
///
/// The source is an async function body rather than an expression, which is
/// what makes those `await`s legal; a leaf ignores what it returns.
///
/// It carries its source rather than a [`Script`](crate::prelude::Script)
/// because a `Script` is generic in its input and output, which BSX has no
/// syntax for, and the point of this action is to be authored in markup. Its
/// sibling [`ScriptExposure`] is the limit on what it may reach.
///
/// A leaf, so `<Repeat>` over it is a ticking dynamic system and `<Sequence>`
/// places it in a larger behaviour.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DynamicScriptAction, ScriptExposure)]
pub struct DynamicScript {
	/// The JavaScript source, evaluated with the `world` API installed.
	pub script: String,
	/// The resources the script may consume before it is cut off.
	pub limits: ScriptLimits,
}

impl DynamicScript {
	/// Create a [`DynamicScript`] from JavaScript source.
	pub fn new(script: impl Into<String>) -> Self {
		Self {
			script: script.into(),
			limits: ScriptLimits::default(),
		}
	}
}

/// Evaluates the caller's [`DynamicScript`] with the `world` bridge installed,
/// then passes.
///
/// The script's completion value is ignored: a leaf answers `Outcome`, and what
/// a bridged script *did* is already in the world. A route
/// (`DynamicScriptRoute`) is the front-end that answers with it.
///
/// ## Errors
/// Errors if the caller has no [`DynamicScript`], or if the script fails to
/// evaluate. A single refused call is not an error here: it rejects inside the
/// script, which may catch it.
#[action(default)]
#[derive(Component)]
pub async fn DynamicScriptAction(cx: ActionContext) -> Result<Outcome> {
	let entity = cx.id();
	let (script, exposure) = cx
		.world()
		.with_state::<Query<(&DynamicScript, &ScriptExposure)>, _>(
			move |scripts| {
				scripts
					.get(entity)
					.map(|(script, exposure)| {
						(script.clone(), exposure.clone())
					})
					.ok()
			},
		)
		.await
		.ok_or_else(|| {
			bevyhow!("DynamicScript caller {entity:?} has no DynamicScript")
		})?;

	Script::<JsonValue, ()>::new(script.script)
		.with_limits(script.limits)
		.run_world(JsonValue::Null, cx.world().clone(), exposure)
		.await?;
	Outcome::PASS.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn world() -> World {
		let world = AsyncPlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();
		world
	}

	async fn run(world: &mut World, script: &str) -> Result<Outcome> {
		world
			.spawn(DynamicScript::new(script))
			.call::<(), Outcome>(())
			.await
	}

	#[beet_core::test]
	async fn spawns_through_the_world_api() {
		let mut world = world();
		run(&mut world, r#"await world.spawn({ "Name": "ada" })"#)
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("ada");
	}

	/// The capability the whole pass is for: the read after the write sees the
	/// write, because both were served against the live world.
	#[beet_core::test]
	async fn a_script_reads_its_own_write() {
		let mut world = world();
		run(
			&mut world,
			r#"
			const entry = await world.spawn({ "Name": "ada" });
			const name = await world.get(entry, "Name");
			await world.insert(entry, "Name", name + " lovelace");
			"#,
		)
		.await
		.unwrap();
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("ada lovelace");
	}

	#[beet_core::test]
	async fn lists_the_entities_carrying_a_component() {
		let mut world = world();
		world.spawn(Name::new("ada"));
		world.spawn(Name::new("bob"));
		run(
			&mut world,
			r#"
			const found = await world.entities("bevy_ecs::name::Name");
			await world.spawn({ "Name": "count:" + found.length });
			"#,
		)
		.await
		.unwrap();
		world
			.query::<&Name>()
			.iter(&world)
			.any(|name| name.as_str() == "count:2")
			.xpect_true();
	}

	#[beet_core::test]
	async fn writes_land_in_evaluation_order() {
		let mut world = world();
		run(
			&mut world,
			r#"
			const entry = await world.spawn({});
			await world.insert(entry, "Name", "one");
			await world.insert(entry, "Name", "two");
			"#,
		)
		.await
		.unwrap();
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("two");
	}

	#[beet_core::test]
	async fn removes_and_despawns() {
		let mut world = world();
		let kept = world.spawn(Name::new("ada")).id();
		world.spawn(Name::new("bob"));
		run(
			&mut world,
			r#"
			for (const id of await world.entities("Name")) {
				if ((await world.get(id, "Name")) === "ada") {
					await world.remove(id, "Name");
				} else {
					await world.despawn(id);
				}
			}
			"#,
		)
		.await
		.unwrap();
		world.entity(kept).contains::<Name>().xpect_false();
		world.query::<&Name>().iter(&world).count().xpect_eq(0);
	}

	/// A refused write rejects the promise at the call site, so the script can
	/// catch it and carry on. That is what makes the exposure a boundary the
	/// script can reason about rather than a trapdoor.
	#[beet_core::test]
	async fn a_refused_write_is_catchable_in_the_script() {
		let mut world = world();
		DynamicComponents::register(
			&mut world,
			"game.Refused",
			ValueSchema::Any,
		)
		.unwrap();
		let entity = world.spawn(Name::new("ada")).id();
		world
			.spawn((
				DynamicScript::new(
					r#"
					const [entry] = await world.entities("Name");
					try {
						await world.insert(entry, "Name", "bob");
					} catch (err) {
						await world.insert(entry, "game.Refused", err.message);
					}
					"#,
				),
				// everything but the name, so the catch block can still record
				// what it was refused
				ScriptExposure {
					write: GlobFilter::default().with_exclude("*Name"),
					..default()
				},
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("ada");
		WorldRead::get(
			&mut world,
			entity,
			"game.Refused",
			&ScriptExposure::default(),
		)
		.unwrap()
		.unwrap()
		.to_string()
		.xpect_contains("may not write");
	}

	/// A script cannot widen its own reach, whatever its exposure says: the
	/// carrier components are refused by rule.
	#[beet_core::test]
	async fn a_script_cannot_rewrite_its_own_exposure() {
		let mut world = world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<ScriptExposure>();
		run(
			&mut world,
			r#"
			const entry = await world.spawn({});
			await world.insert(entry, "ScriptExposure", {});
			"#,
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("no script may write");
	}

	/// The value a script left on `entity`.
	fn read(world: &mut World, entity: Entity, ident: &str) -> Value {
		WorldRead::get(world, entity, ident, &ScriptExposure::default())
			.unwrap()
			.unwrap()
	}

	/// A world with `game.Note` to record in and one named entity to work on.
	fn world_with(declarations: [(&str, ValueSchema); 1]) -> (World, Entity) {
		let mut world = world();
		for (name, schema) in declarations {
			DynamicComponents::register(&mut world, name, schema).unwrap();
		}
		DynamicComponents::register(&mut world, "game.Note", ValueSchema::Any)
			.unwrap();
		let entity = world.spawn(Name::new("ada")).id();
		(world, entity)
	}

	/// A declared schema is a contract the bridge enforces, and a rejection is
	/// the same catchable error an exposure refusal is: the script sees one
	/// shape for "the host said no".
	#[beet_core::test]
	async fn a_declared_schema_rejects_a_non_conforming_value() {
		let (mut world, entity) =
			world_with([("game.Health", ValueSchema::U64(default()))]);
		run(
			&mut world,
			r#"
			const [entry] = await world.entities("Name");
			await world.insert(entry, "game.Health", 3);
			try {
				await world.insert(entry, "game.Health", "three");
			} catch (err) {
				await world.insert(entry, "game.Note", err.message);
			}
			"#,
		)
		.await
		.unwrap();
		// the rejected value never reached the component's storage
		read(&mut world, entity, "game.Health").xpect_eq(Value::Uint(3));
		read(&mut world, entity, "game.Note")
			.to_string()
			.xpect_contains("`game.Health` does not accept this value");
	}

	/// The default is genuinely inert: an undeclared component takes whatever a
	/// document field can hold, exactly as it did before schemas existed.
	#[beet_core::test]
	async fn an_any_schema_accepts_everything() {
		let (mut world, entity) = world_with([("game.Loot", ValueSchema::Any)]);
		run(
			&mut world,
			r#"
			const [entry] = await world.entities("Name");
			for (const value of [1, "sword", [1, 2], { held: true }, null]) {
				await world.insert(entry, "game.Loot", value);
			}
			"#,
		)
		.await
		.unwrap();
		read(&mut world, entity, "game.Loot").xpect_eq(Value::Null);
	}

	/// A schema the world has to resolve before it can decide, which is the
	/// case the async executor exists for: the operation reads the declaration
	/// under one exclusive section, lets go, and only then validates.
	#[beet_core::test]
	async fn validates_against_a_schema_the_world_resolved() {
		let (mut world, entity) = world_with([(
			"game.Health",
			ValueSchema::Reference("game.HealthValue".into()),
		)]);
		let mut registry = SchemaRegistry::default();
		registry.insert("game.HealthValue", ValueSchema::U64(default()));
		world.insert_resource(registry);
		run(
			&mut world,
			r#"
			const [entry] = await world.entities("Name");
			await world.insert(entry, "game.Health", 3);
			try {
				await world.insert(entry, "game.Health", "three");
			} catch (err) {
				await world.insert(entry, "game.Note", err.message);
			}
			"#,
		)
		.await
		.unwrap();
		read(&mut world, entity, "game.Health").xpect_eq(Value::Uint(3));
		read(&mut world, entity, "game.Note")
			.to_string()
			.xpect_contains("does not accept this value");
	}

	/// `world.schema` answers a runtime component with its declaration, which
	/// is the contract every write to it is checked against.
	#[beet_core::test]
	async fn reads_the_declared_schema() {
		let (mut world, entity) =
			world_with([("game.Health", ValueSchema::U64(default()))]);
		run(
			&mut world,
			r#"
			const [entry] = await world.entities("Name");
			const schema = await world.schema("game.Health");
			await world.insert(entry, "game.Note", JSON.stringify(schema));
			"#,
		)
		.await
		.unwrap();
		read(&mut world, entity, "game.Note")
			.to_string()
			.xpect_contains("game.Health")
			.xpect_contains("U64");
	}

	/// JavaScript has one number type and [`Value`] has three, so what a script
	/// reads back must be what it wrote. Pinned before any numeric schema can
	/// act on the distinction.
	#[beet_core::test]
	async fn round_trips_numbers_through_a_script() {
		let (mut world, entity) =
			world_with([("game.Number", ValueSchema::Any)]);
		run(
			&mut world,
			r#"
			const [entry] = await world.entities("Name");
			const sent = [0, -1, 1.5, 9007199254740992];
			const seen = [];
			for (const value of sent) {
				await world.insert(entry, "game.Number", value);
				seen.push(await world.get(entry, "game.Number"));
			}
			await world.insert(entry, "game.Note", { sent, seen });
			"#,
		)
		.await
		.unwrap();
		let note = read(&mut world, entity, "game.Note");
		let note = note.as_map().unwrap();
		note.get("seen")
			.unwrap()
			.xpect_eq(note.get("sent").unwrap().clone())
			.as_list()
			.unwrap()
			.xpect_eq(vec![
				Value::Uint(0),
				Value::Int(-1),
				Value::Float(1.5),
				Value::Uint(9007199254740992),
			]);
	}

	/// A script can hold bytes. JSON has no byte type, so they cross the wire
	/// as a list of numbers and the *destination* restores the type: a
	/// [`Bytes`](ValueSchema::Bytes) schema coerces what arrives, so bytes read
	/// back and written again land as the identical bytes.
	#[beet_core::test]
	async fn round_trips_bytes_through_a_script() {
		let (mut world, entity) =
			world_with([("game.Blob", ValueSchema::Bytes(default()))]);
		run(
			&mut world,
			r#"
			const [entry] = await world.entities("Name");
			await world.insert(entry, "game.Blob", [1, 2, 3, 255]);
			const read = await world.get(entry, "game.Blob");
			await world.insert(entry, "game.Blob", read);
			await world.insert(entry, "game.Note", read);
			"#,
		)
		.await
		.unwrap();
		read(&mut world, entity, "game.Blob")
			.xpect_eq(Value::Bytes(vec![1, 2, 3, 255]));
		// what the script saw is the number list json can carry, which is what
		// makes the destination-directed coercion necessary
		read(&mut world, entity, "game.Note").xpect_eq(Value::List(vec![
			Value::Uint(1),
			Value::Uint(2),
			Value::Uint(3),
			Value::Uint(255),
		]));
	}

	/// The bridge is opt-in surface, not ambient authority: a plain `Script` has
	/// no `world` to reach for, on any backend.
	#[beet_core::test]
	async fn a_pure_script_has_no_world_global() {
		Script::<(), String>::new("typeof world")
			.run(())
			.await
			.unwrap()
			.xpect_eq("undefined".to_string());
	}

	/// The bridge is served inside one sync point, not one call per frame.
	///
	/// [`AsyncPlugin`]'s `max_async_ticks_per_sync_point` drives up to 100
	/// request/wake/poll rounds per sync point, and a bridged call is one
	/// round, so a script making a dozen of them settles inside a single
	/// update. Without that a `<Repeat>` over a `<DynamicScript>` would
	/// silently become one world call per frame.
	///
	/// Embedded engine only: an out-of-process backend is bounded by its
	/// transport (a process spawn, a `postMessage` round trip), not by the tick
	/// budget, so the question this pins does not arise there.
	#[cfg(feature = "quickjs")]
	#[beet_core::test]
	async fn a_dozen_calls_settle_in_one_update() {
		let mut world = world();
		let script = (0..12)
			.map(|index| {
				format!(r#"await world.spawn({{"Name":"n{index}"}});"#)
			})
			.collect::<Vec<_>>()
			.join("\n");
		let entity = world.spawn(DynamicScript::new(script)).id();
		let (send, recv) = OnceValue::oneshot();
		world
			.entity_mut(entity)
			.run_async(move |entity| async move {
				send.signal(entity.call::<(), Outcome>(()).await);
			});
		let mut settled = Box::pin(recv.wait());
		let mut updates = 0;
		let outcome = loop {
			world.update_local();
			updates += 1;
			if let Some(outcome) =
				futures_lite::future::poll_once(&mut settled).await
			{
				break Some(outcome);
			}
			if updates == 10 {
				break None;
			}
		};
		outcome.unwrap().unwrap().xpect_eq(Outcome::PASS);
		updates.xpect_eq(1);
		world.query::<&Name>().iter(&world).count().xpect_eq(12);
	}

	/// The console still works alongside the bridge: installing one channel must
	/// not displace the other.
	#[beet_core::test]
	async fn the_console_survives_the_bridge() {
		let mut world = world();
		run(
			&mut world,
			r#"
			console.log("working");
			await world.spawn({ "Name": "ada" });
			"#,
		)
		.await
		.unwrap();
		world.query::<&Name>().iter(&world).count().xpect_eq(1);
	}
}

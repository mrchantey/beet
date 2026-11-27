//! Types declared by the crates below, through the toolchain: authored from
//! markup, saved and loaded.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;

beet_core::test_main!();

/// Trait for types whose action handler can be constructed statically,
/// allowing a wrapper to build the inner action at `#[require]` time.
trait InnerTestAction: 'static + Send + Sync {
	fn inner_action() -> Action<i32, i32>;
}

/// Inner action defined with `#[action]`, used as the wrapped target.
#[action(pure)]
#[derive(Debug, Reflect)]
fn Doubler(val: i32) -> i32 { val * 2 }

impl InnerTestAction for Doubler {
	fn inner_action() -> Action<i32, i32> { Doubler.into_action() }
}

/// Wrapper function defined with `#[action]`, provides the wrapping logic.
#[action]
async fn AddOneWrap(cx: ActionContext<(i32, Next<i32, i32>)>) -> Result<i32> {
	let inner_out = cx.1.call(cx.0).await?;
	Ok(inner_out + 1)
}

/// Serializable wrapper component, generic over the inner action `T`.
/// Uses `#[require]` to provide the wrapped action at bundle-resolution
/// time, avoiding timing issues during world serde deserialization.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Action<i32, i32> = AddOneWrapper::<T>::make_action())]
struct AddOneWrapper<T: InnerTestAction = Doubler>(
	#[reflect(ignore)] PhantomData<fn() -> T>,
);

impl<T: InnerTestAction> Default for AddOneWrapper<T> {
	fn default() -> Self { Self(PhantomData) }
}

impl<T: InnerTestAction> AddOneWrapper<T> {
	fn make_action() -> Action<i32, i32> {
		let inner = T::inner_action();
		Action::new_async(move |cx: ActionContext<i32>| {
			let inner = inner.clone();
			async move {
				let inner_out = cx.caller.call_detached(inner, *cx).await?;
				Ok(inner_out + 1)
			}
		})
	}
}

fn serde_app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AsyncPlugin, ScenePlugin));
	app.register_type::<AddOneWrapper<Doubler>>();
	app.register_type::<i32>();
	app.init();
	app.update();
	app
}

#[beet_core::test]
async fn wrapper_works_directly() {
	let mut app = serde_app();
	// Only AddOneWrapper is needed — its #[require] provides the
	// wrapped action, constructing the inner Doubler action statically.
	let entity = app
		.world_mut()
		.spawn(AddOneWrapper::<Doubler>::default())
		.id();
	app.update();

	// double(5) + 1 = 11
	app.world_mut()
		.entity_mut(entity)
		.call::<i32, i32>(5)
		.await
		.unwrap()
		.xpect_eq(11);
}

#[beet_core::test]
fn wrapper_template_roundtrip() {
	let mut app = serde_app();

	let entity = app
		.world_mut()
		.spawn(AddOneWrapper::<Doubler>::default())
		.id();
	app.update();

	// The entity should have the wrapper component and a ActionMeta.
	app.world()
		.entity(entity)
		.get::<AddOneWrapper<Doubler>>()
		.xpect_some();
	app.world().entity(entity).get::<ActionMeta>().xpect_some();

	// Serialize
	let template_bytes = TemplateSaver::new()
		.with_entities([entity])
		.save(app.world(), MediaType::Json)
		.unwrap();
	template_bytes
		.as_utf8()
		.unwrap()
		.xref()
		.xpect_contains("AddOneWrapper");

	// Despawn original
	app.world_mut().entity_mut(entity).despawn();
	app.update();

	// Deserialize
	let loaded = TemplateLoader::new(app.world_mut())
		.load(&template_bytes)
		.unwrap();
	app.update();

	// The loaded entity should have the wrapper and a ActionMeta
	// (Action itself isn't serializable, but #[require] re-creates it)
	let loaded = *loaded.first().unwrap();
	app.world()
		.entity(loaded)
		.get::<AddOneWrapper<Doubler>>()
		.xpect_some();
	app.world().entity(loaded).get::<ActionMeta>().xpect_some();
}

#[beet_core::test]
async fn wrapper_works_after_roundtrip() {
	let mut app = serde_app();

	let entity = app
		.world_mut()
		.spawn(AddOneWrapper::<Doubler>::default())
		.id();
	app.update();

	// Serialize then despawn
	let template_bytes = TemplateSaver::new()
		.with_entities([entity])
		.save(app.world(), MediaType::Json)
		.unwrap();
	app.world_mut().entity_mut(entity).despawn();
	app.update();

	// Deserialize
	let loaded = TemplateLoader::new(app.world_mut())
		.load(&template_bytes)
		.unwrap();
	app.update();

	// Call the wrapped action on the loaded entity: double(5) + 1 = 11
	let loaded = *loaded.first().unwrap();
	app.world_mut()
		.entity_mut(loaded)
		.call::<i32, i32>(5)
		.await
		.unwrap()
		.xpect_eq(11);
}

/// A runtime component is saved under its name and loaded back once the
/// name is minted, whichever comes first: here the declaration sits after
/// the entity using it, so the value waits held until it runs.
#[beet_core::test]
fn a_runtime_component_round_trips_a_scene() {
	// a world saving the hierarchy and the declaration
	let scene_world = || {
		let world = (AsyncPlugin, ScenePlugin).into_world();
		let registry = world.resource::<AppTypeRegistry>().clone();
		registry.write().register::<Name>();
		registry.write().register::<DynamicComponent>();
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
	WorldRead::get(&mut loaded, looted, "game.Loot", &ScriptConfig::default())
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

/// The world `markup` loads into, under the plugin that registers the
/// account.
fn load(markup: &str) -> Result<World> {
	let mut world =
		(AsyncPlugin, ScenePlugin, DocumentPlugin, AtprotoPlugin).into_world();
	BsxParser::spawn(&mut world, markup)?;
	world.flush();
	world.xok()
}

/// The account authors from markup, and its PDS lands beside it.
#[beet_core::test]
fn authors_from_markup() {
	let mut world = load(
		r#"<AtprotoAccount handle="beet.org" did="did:plc:vgxy56shhs3t3b2mu2avizjf"
			display_name="Beet" secret="BEET_ORG_APP_PASSWORD"/>"#,
	)
	.unwrap();
	let (entity, account) = world
		.query::<(Entity, &AtprotoAccount)>()
		.single(&world)
		.map(|(entity, account)| (entity, account.clone()))
		.unwrap();
	account
		.did()
		.as_str()
		.xpect_eq("did:plc:vgxy56shhs3t3b2mu2avizjf");
	account
		.secret()
		.clone()
		.xpect_eq(Some("BEET_ORG_APP_PASSWORD".into()));
	PdsQuery::resolve(&mut world, entity)
		.unwrap()
		.id()
		.xpect_eq(XrpcPds::ID);
}

/// A malformed did refuses the load rather than declaring an account no
/// PDS answers for.
#[beet_core::test]
fn refuses_a_malformed_did() {
	load(r#"<AtprotoAccount handle="beet.org" did="beet.org"/>"#)
		.unwrap_err()
		.to_string()
		.xpect_contains("not a did");
}

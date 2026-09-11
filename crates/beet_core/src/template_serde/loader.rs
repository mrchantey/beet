//! Deserializing bytes into a [`DynamicTemplate`] and building it through the
//! [`spawn_template`](WorldTemplateExt::spawn_template) path.
//!
//! [`TemplateLoader`] dispatches by [`MediaType`] to the right serde format,
//! producing a [`DynamicTemplate`], then builds it. Every spawned entity is
//! collected through the build sink, never a second remapping model. When a
//! target entity is given, the spawned roots become its children, after any it
//! already has and before the batch signal fires, so a listener sees the whole
//! hierarchy.

#[cfg(feature = "bsx")]
use crate::prelude::BsxTemplate;
use crate::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;

/// Deserializes template bytes into the world and builds the result.
///
/// With a target entity (via [`TemplateLoader::with_entity`] or
/// [`TemplateLoader::new_entity`]) the spawned roots become children of that
/// entity.
pub struct TemplateLoader<'a> {
	world: &'a mut World,
	/// If set, spawned roots are parented under this entity.
	entity: Option<Entity>,
}

impl<'a> TemplateLoader<'a> {
	/// Creates a loader for the given world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			entity: None,
		}
	}

	/// Creates a loader for the world containing the given entity, parenting
	/// spawned roots under it.
	pub fn new_entity(entity: EntityWorldMut<'a>) -> Self {
		let id = entity.id();
		Self {
			world: entity.into_world_mut(),
			entity: Some(id),
		}
	}

	/// Parents spawned roots under the given entity.
	pub fn with_entity(mut self, entity: Entity) -> Self {
		self.entity = Some(entity);
		self
	}

	/// Loads an entry from [`MediaBytes`] into the world, dispatching by media
	/// type: `.bsx`/`.html` parse to the BSX IR, every serde format
	/// (RON/JSON/postcard) to the [`DynamicTemplate`] IR. Both build through the
	/// one [`spawn_template`](WorldTemplateExt::spawn_template) path.
	pub fn load(self, bytes: &MediaBytes) -> Result<Vec<Entity>> {
		let entry = EntryTemplate::from_bytes(self.world, bytes)?;
		self.load_entry(entry)
	}

	/// Builds a parsed [`EntryTemplate`], retaining a serde document's file keys
	/// on its first spawned root, so a later save writes every entity back under
	/// the key it loaded from.
	pub fn load_entry(self, entry: EntryTemplate) -> Result<Vec<Entity>> {
		let Self { world, entity } = self;
		let (spawned, entity_map) = Self::build(world, entity, entry)?;
		// a markup build feeds no keys: a `.bsx` file is authored original state,
		// never a document the editor rewrites.
		if !entity_map.is_empty() {
			world.entity_mut(spawned[0]).insert(entity_map);
		}
		Ok(spawned)
	}

	/// Builds an [`EntryTemplate`] through
	/// [`spawn_template`](WorldTemplateExt::spawn_template), returning every
	/// spawned entity and the file keys they built from (empty for markup), for
	/// the caller to retain wherever its document lives.
	pub(super) fn build(
		world: &mut World,
		target: Option<Entity>,
		entry: EntryTemplate,
	) -> Result<(Vec<Entity>, TemplateEntityMap)> {
		let is_serde = entry.is_serde();
		// install the sink so a serde build records every real entity it maps to.
		world.insert_resource(TemplateBuildSink::default());
		let root = world.spawn_template(entry)?.id();
		let pairs = world
			.remove_resource::<TemplateBuildSink>()
			.map(|sink| sink.0)
			.unwrap_or_default();
		let entity_map = TemplateEntityMap::from_pairs(pairs.iter().copied());
		let mut spawned = pairs
			.into_iter()
			.map(|(_, entity)| entity)
			.collect::<Vec<_>>();
		// a markup build does not feed the serde sink, so its single spawned root
		// is the whole result.
		if spawned.is_empty() {
			spawned.push(root);
		}

		// in entity mode the spawned roots (no `ChildOf`) join the target's
		// children, after any it already has, and before the batch signal so a
		// listener resolving by ancestry (a route tree's namespace) sees them
		// under their parent.
		if let Some(parent) = target {
			for spawned_entity in spawned.iter() {
				if !world.entity(*spawned_entity).contains::<ChildOf>() {
					world.entity_mut(*spawned_entity).insert(ChildOf(parent));
				}
			}
		}

		// the serde batch-completion signal: reflect inserts settle per-entity, so
		// per-insert observers run before relationships like `ChildOf` are whole.
		// Listeners (eg `RouteTree` rebuild) react once the hierarchy is settled. A
		// markup build inserts in tree order and rebuilds incrementally, so it needs
		// no batch signal.
		if is_serde {
			world.trigger(LoadTemplateSerde {
				entities: spawned.clone(),
			});
		}

		Ok((spawned, entity_map))
	}
}

/// The closed set of entry-document IRs the unified [`TemplateLoader`] builds: the
/// serde [`DynamicTemplate`] or the BSX [`BsxTemplate`].
///
/// Both are [`Template`]s that tail through the one
/// [`spawn_template`](WorldTemplateExt::spawn_template) build path. This enum lets
/// the loader dispatch by [`MediaType`] without a `dyn Template`, which is not
/// object-safe (`Template::clone_template` returns `Self`). They stay distinct
/// IRs: `DynamicTemplate` is the serde round-trip form, `BsxTemplate` a syntax
/// tree resolved against live registries, with no serializable value form.
pub enum EntryTemplate {
	/// The serde IR, from RON/JSON/postcard bytes.
	Serde(super::DynamicTemplate),
	/// The BSX markup IR, from `.bsx`/`.html` source.
	#[cfg(feature = "bsx")]
	Bsx(BsxTemplate),
}

impl EntryTemplate {
	/// Parse entry bytes into the matching IR, dispatching by [`MediaType`]: the
	/// parse half of [`TemplateLoader::load`]. For a caller that builds the result
	/// into an existing entity rather than spawning a root — an include
	/// (`<Template src>`) builds the parsed entry at its own site.
	pub fn from_bytes(world: &World, bytes: &MediaBytes) -> Result<Self> {
		match bytes.media_type() {
			// markup entries parse through the core BSX engine; HTML is the
			// features-off subset of the same grammar.
			#[cfg(feature = "bsx")]
			MediaType::Bsx | MediaType::Html => {
				BsxTemplate::parse_entry(world, bytes.as_utf8()?).map(Self::Bsx)
			}
			// every other type is a serde scene format.
			_ => deserialize_dynamic_template(world, bytes).map(Self::Serde),
		}
	}

	/// Whether this is the serde IR, gating the [`LoadTemplateSerde`] batch signal.
	pub fn is_serde(&self) -> bool {
		match self {
			Self::Serde(_) => true,
			#[cfg(feature = "bsx")]
			Self::Bsx(_) => false,
		}
	}
}

/// Deserialize entry bytes into a [`DynamicTemplate`], dispatching by serde
/// format. The serde half of [`EntryTemplate::from_bytes`].
#[cfg(any(feature = "ron", feature = "json", feature = "postcard"))]
fn deserialize_dynamic_template(
	world: &World,
	bytes: &MediaBytes,
) -> Result<super::DynamicTemplate> {
	use super::serde::DynamicTemplateDeserializer;
	use serde::de::DeserializeSeed;
	let type_registry = world.resource::<AppTypeRegistry>().clone();
	let registry = type_registry.read();
	match bytes.media_type() {
		MediaType::Ron => {
			cfg_if! {
				if #[cfg(feature = "ron")] {
					let text = bytes.as_utf8()?;
					let mut de = ron::de::Deserializer::from_str(text)?;
					DynamicTemplateDeserializer { type_registry: &registry }
						.deserialize(&mut de)?
						.xok()
				} else {
					bevybail!("The `ron` feature is required for RON loading")
				}
			}
		}
		MediaType::Json => {
			cfg_if! {
				if #[cfg(feature = "json")] {
					let mut de = serde_json::Deserializer::from_slice(bytes.bytes());
					DynamicTemplateDeserializer { type_registry: &registry }
						.deserialize(&mut de)?
						.xok()
				} else {
					bevybail!("The `json` feature is required for JSON loading")
				}
			}
		}
		MediaType::Postcard | MediaType::Bytes => {
			cfg_if! {
				if #[cfg(feature = "postcard")] {
					let mut de = postcard::Deserializer::from_bytes(bytes.bytes());
					DynamicTemplateDeserializer { type_registry: &registry }
						.deserialize(&mut de)?
						.xok()
				} else {
					bevybail!("The `postcard` feature is required for postcard loading")
				}
			}
		}
		other => {
			bevybail!("Unsupported media type for template loading: {other}")
		}
	}
}

#[cfg(not(any(feature = "ron", feature = "json", feature = "postcard")))]
fn deserialize_dynamic_template(
	_world: &World,
	_bytes: &MediaBytes,
) -> Result<super::DynamicTemplate> {
	bevybail!(
		"No serde format feature enabled; enable `ron`, `json`, or `postcard`"
	)
}

impl Template for EntryTemplate {
	type Output = ();

	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		match self {
			Self::Serde(template) => template.build_template(cx),
			#[cfg(feature = "bsx")]
			Self::Bsx(template) => template.build_template(cx),
		}
	}

	fn clone_template(&self) -> Self {
		match self {
			Self::Serde(template) => Self::Serde(template.clone_template()),
			#[cfg(feature = "bsx")]
			Self::Bsx(template) => Self::Bsx(template.clone_template()),
		}
	}
}

/// Triggered after a [`TemplateLoader`] builds a whole batch of deserialized
/// entities into the world.
///
/// Distinct from the per-root
/// [`Ready`](crate::prelude::Ready) lifecycle event: this is the
/// batch-completion signal carrying every entity the loader spawned. Reflect
/// loads insert components one entity at a time, so per-insert observers run
/// before relationships settle; listeners use this to react to a completed load
/// synchronously, against the fully-formed hierarchy.
#[derive(Debug, Clone, Event)]
pub struct LoadTemplateSerde {
	/// The entities spawned by this load.
	pub entities: Vec<Entity>,
}

#[cfg(all(test, feature = "bsx", feature = "json"))]
mod entry_test {
	use crate::prelude::*;

	/// A `.bsx` entry and a `.json` entry both load through the one
	/// [`TemplateLoader`] entry point, each landing the expected component on the
	/// spawned root: markup dispatches to the BSX IR, json to the serde IR.
	#[crate::test]
	fn loads_bsx_and_json_through_one_loader() {
		// markup: the single root element's component lands on the spawned root.
		let mut world = TemplatePlugin::world();
		let bsx =
			MediaBytes::new_bsx("<main class=\"app\"><span>hi</span></main>");
		let roots = TemplateLoader::new(&mut world).load(&bsx).unwrap();
		world
			.entity(roots[0])
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("main");

		// serde: a json scene authored by saving a `Name` root, loaded through the
		// same entry point.
		let json = {
			let mut src = TemplatePlugin::world();
			src.resource::<AppTypeRegistry>().write().register::<Name>();
			let root = src.spawn(Name::new("root")).id();
			TemplateSaver::new()
				.with_entity_tree(&src, root)
				.save(&src, MediaType::Json)
				.unwrap()
		};
		let mut world = TemplatePlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();
		let roots = TemplateLoader::new(&mut world).load(&json).unwrap();
		world
			.entity(roots[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("root");
	}
}

#[cfg(all(test, feature = "ron"))]
mod test {
	use crate::prelude::*;

	fn serde_world() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		// `Name`, plus the relationship that carries hierarchy through a
		// round-trip and its `Children` mirror: registering `Children` would let
		// a naive extractor serialize it and double-apply the hierarchy, so this
		// guards that the builder skips the mirror and rebuilds it from `ChildOf`
		// hooks. It also marks the clocks derived.
		app.init_plugin::<MinimalTypesPlugin>();
		app.init();
		app.update();
		app
	}

	/// A whole-world dump round-trips, and carries no clock: [`Time`] is derived
	/// state, marked once at registration rather than denied by this save site.
	#[crate::test]
	fn round_trip_ron() {
		let mut app = serde_world();
		let bytes = TemplateSaver::new_all(app.world())
			.save(app.world(), MediaType::Ron)
			.unwrap();
		bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xnot()
			.xpect_contains("Time");
		TemplateLoader::new(app.world_mut()).load(&bytes).unwrap();
	}

	#[crate::test]
	fn entity_scope() {
		let mut app = serde_world();
		let entity = app.world_mut().spawn(Name::new("Root")).id();
		app.world_mut()
			.entity_mut(entity)
			.with_child(Name::new("Child"));

		let bytes = TemplateSaver::new()
			.with_entity_tree(app.world(), entity)
			.save(app.world(), MediaType::Ron)
			.unwrap();
		let text = bytes.as_utf8().unwrap();
		text.xref().xpect_contains("Root");
		text.xref().xpect_contains("Child");
	}

	/// A targeted load parents the spawned root under the target, after the
	/// children it already had.
	#[crate::test]
	fn loads_into_entity_as_a_child() {
		let mut app = serde_world();
		let child = app.world_mut().spawn(Name::new("TemplateChild")).id();
		let bytes = TemplateSaver::new()
			.with_entities([child])
			.save(app.world(), MediaType::Ron)
			.unwrap();

		let target = app
			.world_mut()
			.spawn((Name::new("Target"), children![Name::new("OldChild")]))
			.id();
		let spawned = TemplateLoader::new(app.world_mut())
			.with_entity(target)
			.load(&bytes)
			.unwrap();

		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("TemplateChild");
		let names = app
			.world()
			.entity(target)
			.get::<Children>()
			.unwrap()
			.iter()
			.map(|child| {
				app.world().entity(child).get::<Name>().unwrap().to_string()
			})
			.collect::<Vec<_>>();
		names.xpect_eq(vec!["OldChild".to_string(), "TemplateChild".into()]);
	}

	/// Children order survives a full save then load round-trip: a parent with
	/// three ordered children is asserted in order after rebuilding.
	#[crate::test]
	fn children_order_round_trips() {
		let mut app = serde_world();
		let root = app
			.world_mut()
			.spawn((Name::new("parent"), children![
				Name::new("a"),
				Name::new("b"),
				Name::new("c"),
			]))
			.id();
		let bytes = TemplateSaver::new()
			.with_entity_tree(app.world(), root)
			.save(app.world(), MediaType::Ron)
			.unwrap();

		let mut app = serde_world();
		let spawned =
			TemplateLoader::new(app.world_mut()).load(&bytes).unwrap();
		// the rebuilt parent is the spawned entity named "parent".
		let parent = spawned
			.iter()
			.copied()
			.find(|entity| {
				app.world()
					.entity(*entity)
					.get::<Name>()
					.map(|name| name.as_str() == "parent")
					.unwrap_or(false)
			})
			.unwrap();
		let order: Vec<String> = app
			.world()
			.entity(parent)
			.get::<Children>()
			.unwrap()
			.iter()
			.map(|child| {
				app.world().entity(child).get::<Name>().unwrap().to_string()
			})
			.collect();
		order.xpect_eq(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
	}
}

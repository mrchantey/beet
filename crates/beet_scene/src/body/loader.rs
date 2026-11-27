//! Reading bytes into a [`SceneBody`] and building it through the
//! [`spawn_template`](WorldTemplateExt::spawn_template) path.
//!
//! [`TemplateLoader`] reads any format's bytes through the world's
//! [`DocParsers`], reflects the body into a [`DynamicTemplate`], then builds
//! it. Every spawned entity is collected through the build sink, never a second
//! remapping model. When a target entity is given, the spawned roots become its
//! children, after any it already has and before the batch signal fires, so a
//! listener sees the whole hierarchy; [`TemplateLoader::load_onto`] builds a
//! body's first entity onto the target itself instead.

use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// Reads document bytes into the world and builds the result.
///
/// With a target entity (via [`TemplateLoader::with_entity`] or
/// [`TemplateLoader::new_entity`]) the spawned roots become children of that
/// entity.
pub struct TemplateLoader<'a> {
	world: &'a mut World,
	/// If set, spawned roots are parented under this entity.
	entity: Option<Entity>,
	/// The package path the document was read from, if it has one.
	path: Option<FwdPath>,
}

impl<'a> TemplateLoader<'a> {
	/// Creates a loader for the given world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			entity: None,
			path: None,
		}
	}

	/// Creates a loader for the world containing the given entity, parenting
	/// spawned roots under it.
	pub fn new_entity(entity: EntityWorldMut<'a>) -> Self {
		let id = entity.id();
		Self::new(entity.into_world_mut()).with_entity(id)
	}

	/// Parents spawned roots under the given entity.
	pub fn with_entity(mut self, entity: Entity) -> Self {
		self.entity = Some(entity);
		self
	}

	/// Loads as the document at `path` of its package, so a [`DocPath`] it
	/// holds resolves from there.
	pub fn with_path(mut self, path: FwdPath) -> Self {
		self.path = Some(path);
		self
	}

	/// Loads a document's [`MediaBytes`] into the world through its
	/// [`DocParsers`].
	pub fn load(self, bytes: &MediaBytes) -> Result<Vec<Entity>> {
		let source =
			DocParsers::of(self.world).read(bytes, self.path.as_ref())?;
		let template = DynamicTemplate::read(self.world, Arc::new(source))?;
		self.load_template(template)
	}

	/// Builds `body`, retaining its file keys on its first spawned root, so a
	/// later save writes every entity back under the key it loaded from.
	pub fn load_body(self, body: &SceneBody) -> Result<Vec<Entity>> {
		let template = self.reflect(body)?;
		self.load_template(template)
	}

	/// [`load_body`](Self::load_body) for a body already reflected.
	fn load_template(self, template: DynamicTemplate) -> Result<Vec<Entity>> {
		let placement = match self.entity {
			Some(parent) => Placement::Under(parent),
			None => Placement::Root,
		};
		let (spawned, entity_map) =
			Self::build(self.world, placement, template)?;
		if let Some(root) = spawned.first() {
			self.world.entity_mut(*root).insert(entity_map);
		}
		Ok(spawned)
	}

	/// Builds `body` onto the loader's entity: the body's first entity is that
	/// entity itself, keeping whatever it already holds, the rest spawn, and
	/// any further root joins its children. The file keys are retained on it.
	///
	/// The reverse of saving an entity's tree, so a tree read back lands where
	/// its reader put it rather than one level below.
	pub fn load_onto(self, body: &SceneBody) -> Result<Vec<Entity>> {
		let template = self.reflect(body)?;
		self.build_onto(template)
	}

	/// [`load_onto`](Self::load_onto) for a body already reflected.
	pub fn build_onto(self, template: DynamicTemplate) -> Result<Vec<Entity>> {
		let root = self.entity.ok_or_else(|| {
			bevyhow!(
				"a scene is loaded onto an entity, see `TemplateLoader::new_entity`"
			)
		})?;
		let (spawned, entity_map) =
			Self::build(self.world, Placement::Onto(root), template)?;
		if !entity_map.is_empty() {
			self.world.entity_mut(root).insert(entity_map);
		}
		Ok(spawned)
	}

	/// `body` reflected as the document it was read from.
	fn reflect(&self, body: &SceneBody) -> Result<DynamicTemplate> {
		DynamicTemplate::reflect_document(self.world, body, self.path.as_ref())
	}

	/// Builds a reflected body through
	/// [`spawn_template`](WorldTemplateExt::spawn_template), returning every
	/// spawned entity and the file keys they built from, for the caller to
	/// retain wherever its document lives.
	pub(super) fn build(
		world: &mut World,
		placement: Placement,
		template: DynamicTemplate,
	) -> Result<(Vec<Entity>, TemplateEntityMap)> {
		// install the sink so the build records every real entity it maps to.
		world.insert_resource(TemplateBuildSink::default());
		match placement {
			Placement::Onto(root) => {
				world.entity_mut(root).insert_template(template)?;
			}
			_ => {
				world.spawn_template(template)?;
			}
		}
		let pairs = world
			.remove_resource::<TemplateBuildSink>()
			.map(|sink| sink.0)
			.unwrap_or_default();
		let entity_map = TemplateEntityMap::from_pairs(pairs.iter().cloned());
		let spawned = pairs
			.into_iter()
			.map(|(_, entity)| entity)
			.collect::<Vec<_>>();

		// in entity mode the spawned roots join the target's children, after any
		// it already has, and before the batch signal so a listener resolving by
		// ancestry (a route tree's namespace) sees them under their parent. A
		// root is an entity nothing in the file owns: neither a parent nor, for
		// an attribute entity, the element it belongs to.
		let parent = match placement {
			Placement::Root => None,
			Placement::Under(parent) | Placement::Onto(parent) => Some(parent),
		};
		if let Some(parent) = parent {
			for spawned_entity in
				spawned.iter().filter(|spawned| **spawned != parent)
			{
				// a filled slot is despawned as the build settles
				let Ok(entity) = world.get_entity(*spawned_entity) else {
					continue;
				};
				if !entity.contains::<ChildOf>()
					&& !entity.contains::<AttributeOf>()
				{
					world.entity_mut(*spawned_entity).insert(ChildOf(parent));
				}
			}
		}

		// the batch-completion signal: reflect inserts settle per-entity, so
		// per-insert observers run before relationships like `ChildOf` are whole.
		// Listeners (eg `RouteTree` rebuild) react once the hierarchy is settled.
		world.trigger(LoadTemplateSerde {
			entities: spawned.clone(),
		});

		Ok((spawned, entity_map))
	}
}

/// Where a load's roots land.
#[derive(Debug, Clone, Copy)]
pub(super) enum Placement {
	/// Spawned with no parent.
	Root,
	/// As children of this entity, after any it already has.
	Under(Entity),
	/// The first entity built onto this entity itself, any further root as its
	/// child.
	Onto(Entity),
}

/// Triggered after a [`TemplateLoader`] builds a whole batch of deserialized
/// entities into the world.
///
/// Distinct from the per-root
/// [`Ready`](beet_core::prelude::Ready) lifecycle event: this is the
/// batch-completion signal carrying every entity the loader spawned. Reflect
/// loads insert components one entity at a time, so per-insert observers run
/// before relationships settle; listeners use this to react to a completed load
/// synchronously, against the fully-formed hierarchy.
#[derive(Debug, Clone, Event)]
pub struct LoadTemplateSerde {
	/// The entities spawned by this load.
	pub entities: Vec<Entity>,
}

#[cfg(all(test, feature = "json"))]
mod entry_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A `.bsx` entry and a `.json` entry both load through the one
	/// [`TemplateLoader`] entry point, each landing the expected component on the
	/// spawned root.
	#[beet_core::test]
	fn loads_bsx_and_json_through_one_loader() {
		// markup: the single root element lands on the spawned root.
		let mut world = ScenePlugin::world();
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
			let mut src = ScenePlugin::world();
			src.resource::<AppTypeRegistry>().write().register::<Name>();
			let root = src.spawn(Name::new("root")).id();
			TemplateSaver::new()
				.with_entity_tree(&src, root)
				.save(&src, MediaType::Json)
				.unwrap()
		};
		let mut world = ScenePlugin::world();
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

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn serde_world() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ScenePlugin));
		// `Name`, plus the relationship that carries hierarchy through a
		// round-trip and its `Children` mirror: registering `Children` would let
		// a naive extractor serialize it and double-apply the hierarchy, so this
		// guards that the builder skips the mirror and rebuilds it from `ChildOf`
		// hooks. It also marks the clocks derived.
		app.install_types::<MinimalTypesPlugin>();
		app.init();
		app.update();
		app
	}

	/// A whole-world dump round-trips, and carries no clock: [`Time`] is derived
	/// state, marked once at registration rather than denied by this save site.
	#[beet_core::test]
	fn round_trip_json() {
		let mut app = serde_world();
		let bytes = TemplateSaver::new_all(app.world())
			.save(app.world(), MediaType::Json)
			.unwrap();
		bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xnot()
			.xpect_contains("Time");
		TemplateLoader::new(app.world_mut()).load(&bytes).unwrap();
	}

	#[beet_core::test]
	fn entity_scope() {
		let mut app = serde_world();
		let entity = app.world_mut().spawn(Name::new("Root")).id();
		app.world_mut()
			.entity_mut(entity)
			.with_child(Name::new("Child"));

		let bytes = TemplateSaver::new()
			.with_entity_tree(app.world(), entity)
			.save(app.world(), MediaType::Json)
			.unwrap();
		let text = bytes.as_utf8().unwrap();
		text.xref().xpect_contains("Root");
		text.xref().xpect_contains("Child");
	}

	/// A targeted load parents the spawned root under the target, after the
	/// children it already had.
	#[beet_core::test]
	fn loads_into_entity_as_a_child() {
		let mut app = serde_world();
		let child = app.world_mut().spawn(Name::new("TemplateChild")).id();
		let bytes = TemplateSaver::new()
			.with_entities([child])
			.save(app.world(), MediaType::Json)
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
	#[beet_core::test]
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
			.save(app.world(), MediaType::Json)
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

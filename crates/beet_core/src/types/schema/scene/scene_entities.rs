//! [`SceneEntities`]: a scene document's entities read as a relation graph,
//! and [`SceneEntitiesMut`], the vocabulary an edit to them needs.
use crate::prelude::*;
use bevy_reflect::TypeRegistry;

/// The `entities` of a scene document (a value of [`ValueSchema::scene`]), read
/// straight off the value as the graph each relation component draws.
///
/// The whole-document twin of the schema walk, which validates one component
/// at a time and so cannot see that a `ChildOf` leads back to its own entity.
/// Consumed twice: the document layer rejects a violating write
/// ([`assert_acyclic`](Self::assert_acyclic)) and an entity picker filters
/// the candidates that would violate it ([`would_cycle`](Self::would_cycle)).
///
/// The one place the entity shape is spelled: every reader goes through this
/// view and every writer through [`SceneEntitiesMut`], so the format can
/// change under them.
#[derive(Debug, Clone, Copy)]
pub struct SceneEntities<'a> {
	entities: &'a Map,
}

impl<'a> SceneEntities<'a> {
	/// The scene's `resources` field.
	pub const RESOURCES: &'static str = "resources";
	/// The scene's `entities` field.
	pub const ENTITIES: &'static str = "entities";
	/// An entity's `components` field.
	pub const COMPONENTS: &'static str = "components";

	/// The entities of `scene`.
	pub fn of(scene: &'a Value) -> Result<Self> {
		scene
			.as_map()?
			.get(Self::ENTITIES)?
			.as_map()?
			.xmap(|entities| Self { entities })
			.xok()
	}

	/// The entities of `scene`, to edit.
	pub fn of_mut(scene: &'a mut Value) -> Result<SceneEntitiesMut<'a>> {
		scene
			.as_map_mut()?
			.0
			.get_mut(Self::ENTITIES)
			.ok_or_else(|| bevyhow!("the scene holds no entities"))?
			.as_map_mut()?
			.xmap(|entities| SceneEntitiesMut { entities })
			.xok()
	}

	/// A scene document holding `entities`, each its component map, in the
	/// given order, and no resources: the fixture every scene test starts
	/// from.
	pub fn scene(entities: impl IntoIterator<Item = (u32, Map)>) -> Value {
		let mut scene = Value::map();
		scene.insert(Self::RESOURCES, Map::default()).unwrap();
		scene.insert(Self::ENTITIES, Map::default()).unwrap();
		let mut view = SceneEntities::of_mut(&mut scene).unwrap();
		for (key, components) in entities {
			view.insert_entity(key, components);
		}
		scene
	}

	/// The raw json of the entity at `key` in a serialized scene: what a
	/// conformance probe indexes, straight off the bytes rather than through
	/// this view, so the format is pinned independently of how it is read.
	/// `Null` where there is none, as json indexing reads.
	#[cfg(feature = "json")]
	pub fn entity_json(
		scene: &serde_json::Value,
		key: u32,
	) -> &serde_json::Value {
		&scene[Self::ENTITIES][key.to_string()][Self::COMPONENTS]
	}

	/// The path of the entity at `key` within its scene document: what a form
	/// over it binds.
	pub fn entity_path(key: u32) -> FieldPath {
		FieldPath::new([
			FieldSegment::key(Self::ENTITIES),
			FieldSegment::key(key.to_string()),
		])
	}

	/// The scene position a field path names: the file key of the entity and
	/// the type path of the component it descends into, ie
	/// `entities.3.components.bevy_ecs::hierarchy::ChildOf[.target]`. `None`
	/// for a path into anything else, which an entity picker takes to mean no
	/// relation to filter by.
	pub fn position(path: &[FieldSegment]) -> Option<(u32, &str)> {
		match path {
			[
				FieldSegment::ObjectKey(entities),
				FieldSegment::ObjectKey(key),
				FieldSegment::ObjectKey(components),
				FieldSegment::ObjectKey(type_path),
				..,
			] if entities == Self::ENTITIES
				&& components == Self::COMPONENTS =>
			{
				Some((key.parse().ok()?, type_path.as_str()))
			}
			_ => None,
		}
	}

	/// The file key of every entity, in **document order**, which is child
	/// order: the build path applies each `ChildOf` in this order, so a parent's
	/// `Children` come out in it.
	pub fn keys(&self) -> Result<Vec<u32>> {
		self.entities
			.0
			.keys()
			.map(|key| {
				key.parse::<u32>().map_err(|_| {
					bevyhow!("entity key `{key}` is not a file key")
				})
			})
			.collect()
	}

	/// The file keys of the entities whose `relation` targets `parent`, in
	/// document order: for `ChildOf`, the `Children` the document describes.
	pub fn related(&self, relation: &str, parent: u32) -> Result<Vec<u32>> {
		self.keys()?
			.into_iter()
			.filter(|key| self.target(*key, relation) == Some(parent))
			.collect::<Vec<_>>()
			.xok()
	}

	/// Whether the scene holds an entity at `key`.
	pub fn contains(&self, key: u32) -> bool {
		self.entities.contains(&key.to_string())
	}

	/// The components of the entity at `key`, if the scene holds one.
	pub fn components(&self, key: u32) -> Option<&'a Map> {
		self.entities
			.0
			.get(key.to_string().as_str())?
			.get(Self::COMPONENTS)?
			.as_map()
			.ok()
	}

	/// The component at `type_path` of the entity at `key`, if it holds one.
	pub fn component(&self, key: u32, type_path: &str) -> Option<&'a Value> {
		self.components(key)?.0.get(type_path)
	}

	/// The entity `key`'s target under `relation` (a component type path), when
	/// it holds one.
	pub fn target(&self, key: u32, relation: &str) -> Option<u32> {
		self.component(key, relation)
			.and_then(EntitySchema::file_key)
	}

	/// Whether pointing `source`'s `relation` at `target` would form a cycle:
	/// the target is the source, or reaches it through the same relation.
	///
	/// What an entity picker filters its candidates by, so a `ChildOf` dropdown
	/// never offers the entity's own descendants.
	pub fn would_cycle(
		&self,
		relation: &str,
		source: u32,
		target: u32,
	) -> bool {
		let mut current = target;
		// a chain longer than the entity count is already a cycle elsewhere
		for _ in 0..=self.entities.0.len() {
			if current == source {
				return true;
			}
			match self.target(current, relation) {
				Some(next) => current = next,
				None => return false,
			}
		}
		true
	}

	/// The entities `source`'s `relation` may target without violating the
	/// [`RelationMeta`] `types` registered for it, in document order: every
	/// entity for a relation free to cycle, else every one that does not lead
	/// back to `source`. An entity picker's options, and the first is the zero
	/// an added reference starts as, so a component added through a picker is
	/// valid before its target is chosen.
	pub fn candidates(
		&self,
		types: &TypeRegistry,
		relation: &str,
		source: u32,
	) -> Result<Vec<u32>> {
		let acyclic =
			RelationMeta::of(types, relation).is_some_and(|meta| meta.acyclic);
		self.keys()?
			.into_iter()
			.filter(|target| {
				!acyclic || !self.would_cycle(relation, source, *target)
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// What the entity at `key` is called: its [`Name`] when it has one, else
	/// its file key and what it most is, ie `#3 div` for an element, `#4 @class`
	/// for an attribute, `#5 "hi"` for a text node, else the first component it
	/// holds that is not an owner relation, so a `<ToggleSceneEditor/>` reads
	/// as one.
	pub fn label(&self, key: u32) -> String {
		let Some(components) = self.components(key) else {
			return format!("#{key}");
		};
		let text = |type_path: &str| {
			components
				.0
				.get(type_path)
				.and_then(|value| value.as_str().ok())
				.map(str::to_string)
		};
		// a name reads alone, unless it is the empty one a freshly added `Name`
		// starts as, which names nothing yet
		if let Some(name) =
			text(Name::type_path()).filter(|name| !name.is_empty())
		{
			return name;
		}
		let kind = if let Some(tag) = text(Element::type_path()) {
			tag
		} else if let Some(attribute) = text(Attribute::type_path()) {
			format!("@{attribute}")
		} else if let Some(value) = text(Value::type_path()) {
			// a text node reads by its opening words, ended with an ellipsis
			// where they were cut, so a paragraph still fits a tree's rail
			let mut preview = value.chars().take(18).collect::<String>();
			if value.chars().count() > 18 {
				preview.push('…');
			}
			format!("{preview:?}")
		} else {
			components
				.0
				.keys()
				.find(|type_path| {
					*type_path != ChildOf::type_path()
						&& *type_path != AttributeOf::type_path()
				})
				.map(|type_path| {
					SchemaRegistry::short_name(type_path).to_string()
				})
				.unwrap_or_default()
		};
		match kind.is_empty() {
			true => format!("#{key}"),
			false => format!("#{key} {kind}"),
		}
	}

	/// Every relation `types` marks [`acyclic`](RelationMeta::acyclic) holds,
	/// else an error naming the relation and both entities of the edge that
	/// closes the cycle.
	///
	/// The document layer's check on a relation write: the world would only
	/// discover the cycle by panicking, so the document refuses it first.
	pub fn assert_acyclic(&self, types: &TypeRegistry) -> Result {
		for key in self.keys()? {
			let Some(components) = self.components(key) else {
				continue;
			};
			for (relation, value) in components.0.iter() {
				// every fact the meta carries is read here, so a new one fails to
				// compile until this check says what it means
				let Some(RelationMeta { acyclic }) =
					RelationMeta::of(types, relation).copied()
				else {
					continue;
				};
				let Some(target) = EntitySchema::file_key(value) else {
					continue;
				};
				if acyclic && self.would_cycle(relation, key, target) {
					match target == key {
						true => bevybail!(
							"relation `{relation}` on entity #{key} targets itself"
						),
						false => bevybail!(
							"relation `{relation}` from entity #{key} to #{target} \
							would cycle: #{target} already leads back to #{key}"
						),
					}
				}
			}
		}
		OK
	}
}

/// The writing twin of [`SceneEntities`]: the edits a scene document takes,
/// each landing in the shape the view reads, so no editor spells it.
#[derive(Debug)]
pub struct SceneEntitiesMut<'a> {
	entities: &'a mut Map,
}

impl<'a> SceneEntitiesMut<'a> {
	/// Insert an entity holding `components` at `key`, last in document order
	/// (so last among its siblings), replacing any entity there.
	pub fn insert_entity(&mut self, key: u32, components: Map) {
		let mut entity = Map::default();
		entity.insert(SceneEntities::COMPONENTS, components);
		self.entities.insert(key.to_string(), entity);
	}

	/// Remove the entity at `key`, answering whether the scene held one. What
	/// it owned stays: an editor removes a subtree by walking the view first.
	pub fn remove_entity(&mut self, key: u32) -> bool {
		self.entities.remove(&key.to_string()).is_some()
	}

	/// Move the entity at `key` to document position `index`, the edit a
	/// reorder makes: document order is child order.
	pub fn move_entity(&mut self, key: u32, index: usize) -> Result {
		let from = self
			.entities
			.0
			.get_index_of(key.to_string().as_str())
			.ok_or_else(|| bevyhow!("the scene holds no entity #{key}"))?;
		self.entities.0.move_index(from, index);
		OK
	}

	/// The components of the entity at `key`, to edit.
	pub fn components_mut(&mut self, key: u32) -> Option<&mut Map> {
		self.entities
			.0
			.get_mut(key.to_string().as_str())?
			.get_mut(SceneEntities::COMPONENTS)?
			.as_map_mut()
			.ok()
	}

	/// The component at `type_path` of the entity at `key`, to edit.
	pub fn component_mut(
		&mut self,
		key: u32,
		type_path: &str,
	) -> Option<&mut Value> {
		self.components_mut(key)?.0.get_mut(type_path)
	}

	/// Set the component at `type_path` of the entity at `key`, adding it last
	/// if the entity lacks one; an error for an entity the scene lacks.
	pub fn insert_component(
		&mut self,
		key: u32,
		type_path: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Result {
		self.components_mut(key)
			.ok_or_else(|| bevyhow!("the scene holds no entity #{key}"))?
			.insert(type_path, value);
		OK
	}

	/// Remove the component at `type_path` from the entity at `key`, answering
	/// it if there was one.
	pub fn remove_component(
		&mut self,
		key: u32,
		type_path: &str,
	) -> Option<Value> {
		self.components_mut(key)?.remove(type_path)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy_reflect::TypeRegistry;

	const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";
	const NAME: &str = "bevy_ecs::name::Name";
	const ELEMENT: &str = "beet_core::types::element::element::Element";
	const ATTRIBUTE: &str = "beet_core::types::snippet::attribute::Attribute";
	const VALUE: &str = "beet_core::types::value::value::Value";

	fn types() -> TypeRegistry {
		let mut types = TypeRegistry::default();
		RelationMeta::acyclic().register::<ChildOf>(&mut types);
		types
	}

	/// The components of a child of `parent`.
	fn child(parent: u32) -> Map {
		Map::new([(CHILD_OF, EntitySchema::reference(parent).unwrap())])
	}

	/// A root with a chain of two children: `0 <- 1 <- 2`.
	fn scene() -> Value {
		SceneEntities::scene([
			(0, Map::default()),
			(1, child(0)),
			(2, child(1)),
		])
	}

	/// Set entity `key`'s parent, the write an editor's `ChildOf` picker lands.
	fn reparent(scene: &mut Value, key: u32, parent: u32) {
		SceneEntities::of_mut(scene)
			.unwrap()
			.insert_component(
				key,
				CHILD_OF,
				EntitySchema::reference(parent).unwrap(),
			)
			.unwrap();
	}

	#[crate::test]
	fn a_tree_is_acyclic() {
		let scene = scene();
		let entities = SceneEntities::of(&scene).unwrap();
		entities.assert_acyclic(&types()).unwrap();
		entities.keys().unwrap().xpect_eq(vec![0, 1, 2]);
		entities.target(2, CHILD_OF).unwrap().xpect_eq(1);
		entities.target(0, CHILD_OF).xpect_none();
	}

	/// The edits land in the shape the view reads, and in document order: an
	/// inserted entity comes last, a moved one where it was put, a removed
	/// component is gone and a removed entity with it.
	#[crate::test]
	fn edits_round_trip_through_the_view() {
		let mut scene = scene();
		let mut entities = SceneEntities::of_mut(&mut scene).unwrap();
		entities.insert_entity(3, child(0));
		entities.insert_component(3, NAME, "c").unwrap();
		entities.insert_component(9, NAME, "nobody").unwrap_err();
		*entities.component_mut(3, NAME).unwrap() = Value::str("d");
		entities.move_entity(3, 1).unwrap();
		entities.move_entity(9, 0).unwrap_err();
		entities.remove_component(1, CHILD_OF).unwrap();
		entities.remove_entity(2).xpect_true();
		entities.remove_entity(2).xpect_false();
		let entities = SceneEntities::of(&scene).unwrap();
		entities.keys().unwrap().xpect_eq(vec![0, 3, 1]);
		entities
			.component(3, NAME)
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("d");
		entities.target(3, CHILD_OF).unwrap().xpect_eq(0);
		entities.target(1, CHILD_OF).xpect_none();
		entities.components(1).unwrap().0.len().xpect_eq(0);
		entities.related(CHILD_OF, 0).unwrap().xpect_eq(vec![3]);
	}

	/// Reparenting the root under its grandchild is rejected naming the
	/// relation and both entities of the write.
	#[crate::test]
	fn a_cyclic_write_is_rejected() {
		let mut scene = scene();
		reparent(&mut scene, 0, 2);
		SceneEntities::of(&scene)
			.unwrap()
			.assert_acyclic(&types())
			.unwrap_err()
			.to_string()
			.xpect_contains(CHILD_OF)
			.xpect_contains("#0")
			.xpect_contains("#2");
	}

	#[crate::test]
	fn a_self_reference_is_rejected() {
		let mut scene = scene();
		reparent(&mut scene, 1, 1);
		SceneEntities::of(&scene)
			.unwrap()
			.assert_acyclic(&types())
			.unwrap_err()
			.to_string()
			.xpect_contains(CHILD_OF)
			.xpect_contains("#1 targets itself");
	}

	/// The picker's question: which candidates would a reparent of `1` cycle
	/// on? Its own subtree and itself, never its parent or a sibling.
	#[crate::test]
	fn a_picker_filters_the_subtree() {
		let mut scene = scene();
		SceneEntities::of_mut(&mut scene)
			.unwrap()
			.insert_entity(3, child(0));
		let entities = SceneEntities::of(&scene).unwrap();
		entities.would_cycle(CHILD_OF, 1, 1).xpect_true();
		entities.would_cycle(CHILD_OF, 1, 2).xpect_true();
		entities.would_cycle(CHILD_OF, 1, 0).xpect_false();
		entities.would_cycle(CHILD_OF, 1, 3).xpect_false();
	}

	/// The picker's options for a reparent of `1`: every entity but itself and
	/// its subtree, the first of which is the zero an added `ChildOf` starts
	/// as; an unmarked relation offers every entity.
	#[crate::test]
	fn candidates_honour_the_relation_meta() {
		let mut scene = scene();
		SceneEntities::of_mut(&mut scene)
			.unwrap()
			.insert_entity(3, child(0));
		let entities = SceneEntities::of(&scene).unwrap();
		entities
			.candidates(&types(), CHILD_OF, 1)
			.unwrap()
			.xpect_eq(vec![0, 3]);
		entities
			.candidates(&TypeRegistry::default(), CHILD_OF, 1)
			.unwrap()
			.xpect_eq(vec![0, 1, 2, 3]);
	}

	/// An entity is called by its name, else by its key and what it most is.
	#[crate::test]
	fn labels_say_what_an_entity_is() {
		let scene = SceneEntities::scene([
			(0, Map::new([(NAME, "root")])),
			(
				1,
				Map::new([
					(ELEMENT, Value::str("div")),
					(CHILD_OF, EntitySchema::reference(0).unwrap()),
				]),
			),
			(2, Map::new([(ATTRIBUTE, "class"), (VALUE, "card")])),
			(3, Map::new([(VALUE, "hello")])),
			(6, Map::new([(VALUE, "a paragraph long enough to cut")])),
			(
				4,
				Map::new([
					(CHILD_OF, EntitySchema::reference(0).unwrap()),
					("my_crate::widgets::Toggle", Value::map()),
				]),
			),
			(5, Map::default()),
		]);
		let entities = SceneEntities::of(&scene).unwrap();
		entities.label(0).xpect_eq("root");
		entities.label(1).xpect_eq("#1 div");
		let mut unnamed = scene.clone();
		SceneEntities::of_mut(&mut unnamed)
			.unwrap()
			.insert_component(1, NAME, "")
			.unwrap();
		SceneEntities::of(&unnamed)
			.unwrap()
			.label(1)
			.xpect_eq("#1 div");
		entities.label(2).xpect_eq("#2 @class");
		entities.label(3).xpect_eq("#3 \"hello\"");
		entities.label(6).xpect_eq("#6 \"a paragraph long e…\"");
		entities.label(4).xpect_eq("#4 Toggle");
		entities.label(5).xpect_eq("#5");
		entities.label(9).xpect_eq("#9");
	}

	/// An unmarked relation stays free to cycle: the meta is opt-in.
	#[crate::test]
	fn an_unmarked_relation_may_cycle() {
		let mut scene = scene();
		reparent(&mut scene, 0, 2);
		SceneEntities::of(&scene)
			.unwrap()
			.assert_acyclic(&TypeRegistry::default())
			.unwrap();
	}
}

//! [`SceneEntities`]: a scene document's entities read as a relation graph.
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
#[derive(Debug, Clone, Copy)]
pub struct SceneEntities<'a> {
	entities: &'a Map,
}

impl<'a> SceneEntities<'a> {
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

	/// The path of the entity at `key` within its scene document.
	pub fn entity_path(key: u32) -> FieldPath {
		FieldPath::new([
			FieldSegment::key(Self::ENTITIES),
			FieldSegment::key(key.to_string()),
		])
	}

	/// The path of the component map of the entity at `key`.
	pub fn components_path(key: u32) -> FieldPath {
		Self::entity_path(key).with_pushed(Self::COMPONENTS)
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

	/// The entity `key`'s target under `relation` (a component type path), when
	/// it holds one.
	pub fn target(&self, key: u32, relation: &str) -> Option<u32> {
		self.components(key)?
			.0
			.get(relation)
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy_reflect::TypeRegistry;

	const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";

	fn types() -> TypeRegistry {
		let mut types = TypeRegistry::default();
		RelationMeta::acyclic().register::<ChildOf>(&mut types);
		types
	}

	/// `{ components: { ChildOf: parent } }`
	fn child(parent: u32) -> Value {
		value!({ "components": {
			"bevy_ecs::hierarchy::ChildOf": (EntitySchema::reference(parent).unwrap())
		} })
	}

	/// A root with a chain of two children: `0 <- 1 <- 2`.
	fn scene() -> Value {
		value!({
			"resources": {},
			"entities": {
				"0": { "components": {} },
				"1": (child(0)),
				"2": (child(1))
			}
		})
	}

	/// Set entity `key`'s parent, the write an editor's `ChildOf` picker lands.
	fn reparent(scene: &mut Value, key: u32, parent: u32) {
		scene
			.get_mut("entities")
			.unwrap()
			.insert(key.to_string(), child(parent))
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
		scene
			.get_mut("entities")
			.unwrap()
			.insert("3", child(0))
			.unwrap();
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
		scene
			.get_mut("entities")
			.unwrap()
			.insert("3", child(0))
			.unwrap();
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
		let scene = value!({
			"resources": {},
			"entities": {
				"0": { "components": { "bevy_ecs::name::Name": "root" } },
				"1": { "components": {
					"beet_core::types::element::element::Element": "div",
					"bevy_ecs::hierarchy::ChildOf": (EntitySchema::reference(0).unwrap())
				} },
				"2": { "components": {
					"beet_core::types::snippet::attribute::Attribute": "class",
					"beet_core::types::value::value::Value": "card"
				} },
				"3": { "components": {
					"beet_core::types::value::value::Value": "hello"
				} },
				"6": { "components": {
					"beet_core::types::value::value::Value": "a paragraph long enough to cut"
				} },
				"4": { "components": {
					"bevy_ecs::hierarchy::ChildOf": (EntitySchema::reference(0).unwrap()),
					"my_crate::widgets::Toggle": {}
				} },
				"5": { "components": {} }
			}
		});
		let entities = SceneEntities::of(&scene).unwrap();
		entities.label(0).xpect_eq("root");
		entities.label(1).xpect_eq("#1 div");
		let mut unnamed = scene.clone();
		unnamed
			.get_mut("entities")
			.unwrap()
			.get_mut("1")
			.unwrap()
			.get_mut("components")
			.unwrap()
			.insert("bevy_ecs::name::Name", "")
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

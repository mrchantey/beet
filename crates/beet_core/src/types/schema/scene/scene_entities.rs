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
	/// The entities of `scene`.
	pub fn of(scene: &'a Value) -> Result<Self> {
		scene
			.as_map()?
			.get("entities")?
			.as_map()?
			.xmap(|entities| Self { entities })
			.xok()
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
			.get("components")?
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

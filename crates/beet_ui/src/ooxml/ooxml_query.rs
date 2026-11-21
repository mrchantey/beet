//! Reading a part's [`OoxmlNode`]s, and the changes a projection makes to
//! them.
use crate::prelude::*;
use beet_core::prelude::*;

/// Read access over the nodes of a part: its elements by namespace and local
/// name, their attributes, and their text. What a projection reads before
/// deciding what each node means.
#[derive(SystemParam)]
pub(crate) struct OoxmlQuery<'w, 's> {
	nodes: Query<'w, 's, &'static OoxmlNode>,
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
	texts:
		Query<'w, 's, &'static Value, (Without<OoxmlNode>, Without<Element>)>,
}

impl OoxmlQuery<'_, '_> {
	/// `entity`'s element, when it is one.
	pub fn element(&self, entity: Entity) -> Option<&OoxmlElement> {
		self.nodes.get(entity).ok().and_then(OoxmlNode::element)
	}

	/// Whether `entity` is a part.
	pub fn is_part(&self, entity: Entity) -> bool {
		self.nodes.get(entity).is_ok_and(OoxmlNode::is_part)
	}

	/// Whether `entity` is `local` in `namespace`.
	pub fn is(&self, entity: Entity, namespace: &str, local: &str) -> bool {
		self.element(entity)
			.is_some_and(|element| element.is(namespace, local))
	}

	/// Whether `entity` is a reader's text node.
	pub fn is_text(&self, entity: Entity) -> bool {
		self.texts.contains(entity)
	}

	pub fn parent(&self, entity: Entity) -> Option<Entity> {
		self.parents.get(entity).ok().map(ChildOf::parent)
	}

	pub fn children(&self, entity: Entity) -> Vec<Entity> {
		self.children
			.get(entity)
			.map(|children| children.to_vec())
			.unwrap_or_default()
	}

	/// The first child `local` in `namespace`.
	pub fn child(
		&self,
		entity: Entity,
		namespace: &str,
		local: &str,
	) -> Option<Entity> {
		self.children(entity)
			.into_iter()
			.find(|child| self.is(*child, namespace, local))
	}

	/// The children `local` in `namespace`, in order.
	pub fn children_named(
		&self,
		entity: Entity,
		namespace: &str,
		local: &str,
	) -> Vec<Entity> {
		self.children(entity)
			.into_iter()
			.filter(|child| self.is(*child, namespace, local))
			.collect()
	}

	/// Every descendant in document order, `entity` excluded.
	pub fn descendants(&self, entity: Entity) -> Vec<Entity> {
		self.children.iter_descendants_depth_first(entity).collect()
	}

	/// Every descendant `local` in `namespace`, in document order.
	pub fn descendants_named(
		&self,
		entity: Entity,
		namespace: &str,
		local: &str,
	) -> Vec<Entity> {
		self.children
			.iter_descendants_depth_first(entity)
			.filter(|descendant| self.is(*descendant, namespace, local))
			.collect()
	}


	/// Every ancestor, the nearest first.
	pub fn ancestors(&self, entity: Entity) -> Vec<Entity> {
		self.parents.iter_ancestors(entity).collect()
	}






	/// The value of `entity`'s attribute `local` in `namespace`, or
	/// unprefixed when `None`.
	pub fn attribute(
		&self,
		entity: Entity,
		namespace: Option<&str>,
		local: &str,
	) -> Option<&str> {
		self.element(entity)?.attribute(namespace, local)
	}



	/// The text of every text node under `entity`, joined.
	pub fn text(&self, entity: Entity) -> String {
		self.children
			.iter_descendants_depth_first(entity)
			.filter_map(|descendant| match self.texts.get(descendant) {
				Ok(Value::Str(text)) => Some(text.as_str()),
				_ => None,
			})
			.collect()
	}
}

/// An HTML element a projection gives a node: its tag and attributes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tagged {
	pub tag: SmolStr,
	pub attributes: Vec<(SmolStr, Value)>,
}

impl Tagged {
	pub fn new(tag: impl Into<SmolStr>) -> Self {
		Self {
			tag: tag.into(),
			attributes: Vec::new(),
		}
	}

	pub fn with(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Self {
		self.attributes.push((key.into(), value.into()));
		self
	}


	/// Inserts the element and spawns its attributes on `entity`.
	pub fn insert(self, world: &mut World, entity: Entity) {
		world.entity_mut(entity).insert(Element::new(self.tag));
		for (key, value) in self.attributes {
			world.spawn((AttributeOf::new(entity), Attribute::new(key), value));
		}
	}
}

/// One change a projection makes to a source tree, decided by reading it
/// whole and applied after, so the read never sees its own writes.
pub(crate) enum Projected {
	/// The node means what this element means.
	Tag(Entity, Tagged),
	/// A text node no reader sees: its value becomes an
	/// [`OoxmlNode::Text`].
	Hide(Entity),
	/// A component the projection reads the node into, ie a slide
	/// paragraph's [`ListLevel`](crate::prelude::ListLevel).
	Insert(Entity, Box<dyn FnOnce(&mut EntityWorldMut)>),
}

impl Projected {
	/// A component for `entity`.
	pub fn insert(entity: Entity, bundle: impl 'static + Bundle) -> Self {
		Self::Insert(
			entity,
			Box::new(move |entity: &mut EntityWorldMut| {
				entity.insert(bundle);
			}),
		)
	}

	/// Applies every change in order.
	pub fn apply(world: &mut World, changes: Vec<Projected>) {
		for change in changes {
			match change {
				Self::Tag(entity, tagged) => tagged.insert(world, entity),
				Self::Hide(entity) => {
					let mut entity = world.entity_mut(entity);
					if let Some(Value::Str(text)) = entity.take::<Value>() {
						entity.insert(OoxmlNode::Text(text));
					}
				}
				Self::Insert(entity, insert) => {
					insert(&mut world.entity_mut(entity));
				}
			}
		}
	}
}

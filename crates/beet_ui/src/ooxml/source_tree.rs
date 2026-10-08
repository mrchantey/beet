//! Reading a part's source tree, and the changes a projection makes to it.

use beet_core::prelude::*;

/// Read access over the source tree of a part: its elements by namespace and
/// local name, their attributes, and their text. What a projection reads
/// before deciding what each node means.
#[derive(SystemParam)]
pub(crate) struct SourceTree<'w, 's> {
	elements: Query<'w, 's, &'static SourceElement>,
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
	texts: Query<
		'w,
		's,
		&'static Value,
		(Without<SourceElement>, Without<Element>),
	>,
}

impl SourceTree<'_, '_> {
	pub fn element(&self, entity: Entity) -> Option<&SourceElement> {
		self.elements.get(entity).ok()
	}

	/// Whether `entity` is `local` in `namespace`.
	pub fn is(&self, entity: Entity, namespace: &str, local: &str) -> bool {
		self.element(entity)
			.is_some_and(|element| element.is(namespace, local))
	}

	/// Whether `entity` is a text node.
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

	/// The first descendant `local` in `namespace`.
	pub fn descendant(
		&self,
		entity: Entity,
		namespace: &str,
		local: &str,
	) -> Option<Entity> {
		self.children
			.iter_descendants_depth_first(entity)
			.find(|descendant| self.is(*descendant, namespace, local))
	}

	/// Every ancestor, the nearest first.
	pub fn ancestors(&self, entity: Entity) -> Vec<Entity> {
		self.parents.iter_ancestors(entity).collect()
	}

	/// The nearest ancestor `local` in `namespace`.
	pub fn ancestor(
		&self,
		entity: Entity,
		namespace: &str,
		local: &str,
	) -> Option<Entity> {
		self.parents
			.iter_ancestors(entity)
			.find(|ancestor| self.is(*ancestor, namespace, local))
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

	/// The attribute `attribute` of `entity`'s property `local`, a child of
	/// its properties element, ie a run's `w:rPr/w:color@w:val`.
	pub fn property(
		&self,
		entity: Entity,
		properties: &str,
		namespace: &str,
		local: &str,
		attribute: &str,
	) -> Option<&str> {
		let properties = self.child(entity, namespace, properties)?;
		let property = self.child(properties, namespace, local)?;
		self.attribute(property, Some(namespace), attribute)
	}

	/// Whether `entity`'s properties carry the toggle `local` switched on, ie
	/// a run's `w:b`, present and not `0` or `false`.
	pub fn toggle(
		&self,
		entity: Entity,
		properties: &str,
		namespace: &str,
		local: &str,
	) -> bool {
		self.child(entity, namespace, properties)
			.and_then(|properties| self.child(properties, namespace, local))
			.is_some_and(|toggle| {
				!matches!(
					self.attribute(toggle, Some(namespace), "val"),
					Some("0" | "false")
				)
			})
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

	/// A flag attribute, present with no value, ie `checked`.
	pub fn flag(mut self, key: impl Into<SmolStr>) -> Self {
		self.attributes.push((key.into(), Value::Null));
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
	/// A text node no reader sees: its value becomes a [`SourceText`].
	Hide(Entity),
	/// A node's own words, ie a tab's space, as its own [`Value`].
	Show(Entity, SmolStr),
	/// Projection-only elements, outermost first, nested under the node and
	/// wrapping `content`, ie a bold run's `<strong>` inside its `<mark>`
	/// around everything after its properties.
	Wrap {
		entity: Entity,
		wrappers: Vec<Tagged>,
		content: Vec<Entity>,
	},
	/// A component the projection reads the node into, ie its
	/// [`RunLook`](crate::prelude::RunLook).
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
						entity.insert(SourceText::new(text.as_str()));
					}
				}
				Self::Show(entity, text) => {
					world.entity_mut(entity).insert(Value::Str(text));
				}
				Self::Wrap {
					entity,
					wrappers,
					content,
				} => {
					let mut parent = entity;
					for wrapper in wrappers {
						let child = world.spawn(ChildOf(parent)).id();
						wrapper.insert(world, child);
						parent = child;
					}
					world.entity_mut(parent).add_children(&content);
				}
				Self::Insert(entity, insert) => {
					insert(&mut world.entity_mut(entity));
				}
			}
		}
	}
}

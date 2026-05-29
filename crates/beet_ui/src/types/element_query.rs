use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QueryEntityError;

#[derive(SystemParam)]
pub struct ElementQuery<'w, 's> {
	elements: Query<
		'w,
		's,
		(
			Entity,
			&'static Element,
			Option<&'static Attributes>,
			Option<&'static ElementStateMap>,
			Option<&'static Classes>,
			Option<&'static Children>,
		),
	>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	attributes: Query<'w, 's, (Entity, &'static Attribute, &'static Value)>,
	values: Query<'w, 's, &'static Value, Without<Element>>,
	/// Used by descendant traversal — picks up `Children` regardless of whether
	/// the entity is an [`Element`] (value/text entities can carry children
	/// too in principle, and we want the same walker to work for both).
	all_children: Query<'w, 's, &'static Children>,
}

impl ElementQuery<'_, '_> {
	fn collect_attributes(
		&self,
		attrs: Option<&Attributes>,
	) -> Vec<AttributeView<'_>> {
		attrs
			.map(|attrs| {
				attrs
					.iter()
					.filter_map(|attr| self.attributes.get(attr).ok())
					.map(|(entity, attribute, value)| AttributeView {
						entity,
						attribute,
						value,
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// The [`Value`] of the sole child when present, ie the text content
	/// of an element whose only child is a text node.
	fn collect_inner_text(
		&self,
		children: Option<&Children>,
	) -> Option<(Entity, &Value)> {
		let children = children?;
		let mut iter = children.iter();
		let only_child = iter.next()?;
		if iter.next().is_some() {
			return None;
		}
		self.values.get(only_child).ok().map(|v| (only_child, v))
	}

	pub fn iter(&self) -> impl Iterator<Item = ElementView<'_>> {
		self.elements.iter().map(
			|(entity, element, attrs, state, classes, children)| {
				let attributes = self.collect_attributes(attrs);
				let inner_text = self.collect_inner_text(children);
				ElementView::new(
					entity, element, attributes, state, classes, inner_text,
				)
			},
		)
	}

	pub fn get(
		&self,
		entity: Entity,
	) -> Result<ElementView<'_>, QueryEntityError> {
		self.elements.get(entity).map(
			|(entity, element, attrs, state, classes, children)| {
				let attributes = self.collect_attributes(attrs);
				let inner_text = self.collect_inner_text(children);
				ElementView::new(
					entity, element, attributes, state, classes, inner_text,
				)
			},
		)
	}
	pub fn get_in_ancestors(
		&self,
		entity: Entity,
	) -> Result<ElementView<'_>, QueryEntityError> {
		match self.get(entity) {
			Ok(val) => val.xok(),
			Err(_) if let Ok(parent) = self.ancestors.get(entity) => {
				self.get_in_ancestors(parent.0)
			}
			Err(err) => Err(err),
		}
	}

	pub fn get_as<'a, T>(&'a self, entity: Entity) -> Result<T>
	where
		T: TypedElementView<'a>,
	{
		let element = self.get(entity)?;
		element.try_as::<T>().map_err(|e| e.into())
	}

	/// Pre-order walk over `root` and every entity reachable via [`Children`],
	/// yielding an [`ElementView`] for each entity that is itself an
	/// [`Element`]. Non-element entities (eg. text [`Value`]s) are skipped, but
	/// their children are still traversed.
	pub fn iter_descendants_inclusive(
		&self,
		root: Entity,
	) -> impl Iterator<Item = ElementView<'_>> {
		let mut stack = vec![root];
		let mut out = Vec::new();
		while let Some(entity) = stack.pop() {
			if let Ok(view) = self.get(entity) {
				out.push(view);
			}
			if let Ok(children) = self.all_children.get(entity) {
				// reverse so we yield children left-to-right
				for child in children.iter().rev() {
					stack.push(child);
				}
			}
		}
		out.into_iter()
	}

	/// Every [`Value`] reachable from `root` via the [`Children`] tree,
	/// excluding values stored on element entities themselves (attributes live
	/// under [`AttributeOf`], not [`ChildOf`], so they aren't visited).
	pub fn iter_descendant_values(
		&self,
		root: Entity,
	) -> impl Iterator<Item = &Value> {
		let mut stack = vec![root];
		let mut out = Vec::new();
		while let Some(entity) = stack.pop() {
			if let Ok(value) = self.values.get(entity) {
				out.push(value);
			}
			if let Ok(children) = self.all_children.get(entity) {
				for child in children.iter().rev() {
					stack.push(child);
				}
			}
		}
		out.into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn get_collects_class_attributes() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Element::new("div"),
				related!(
					Attributes[(
						Attribute::new("class"),
						Value::str("hero light-scheme")
					)]
				),
			))
			.id();

		world.with_state::<ElementQuery, _>(|query| {
			let element = query.get(entity).unwrap();
			element
				.attribute("class")
				.unwrap()
				.value
				.as_str()
				.unwrap()
				.xpect_eq("hero light-scheme");
			element.contains_class("hero").xpect_true();
			element.contains_class("light-scheme").xpect_true();
			element.contains_class("dark-scheme").xpect_false();
		});
	}

	#[beet_core::test]
	fn iter_preserves_class_attributes() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Element::new("button"),
				related!(
					Attributes[(
						Attribute::new("class"),
						Value::str("primary filled")
					)]
				),
			))
			.id();

		world.with_state::<ElementQuery, _>(|query| {
			let element = query.iter().find(|el| el.entity == entity).unwrap();
			element.contains_class("primary").xpect_true();
			element.contains_class("filled").xpect_true();
		});
	}
}

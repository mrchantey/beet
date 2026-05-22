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

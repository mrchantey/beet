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
		),
	>,
	attributes: Query<'w, 's, (Entity, &'static Attribute, &'static Value)>,
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

	pub fn iter(&self) -> impl Iterator<Item = ElementView<'_>> {
		self.elements.iter().map(|(entity, element, attrs, state)| {
			let attributes = self.collect_attributes(attrs);
			ElementView::new(entity, element, attributes, state)
		})
	}

	pub fn get(
		&self,
		entity: Entity,
	) -> Result<ElementView<'_>, QueryEntityError> {
		self.elements
			.get(entity)
			.map(|(entity, element, attrs, state)| {
				let attributes = self.collect_attributes(attrs);
				ElementView::new(entity, element, attributes, state)
			})
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

	#[test]
	fn get_collects_class_attributes() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Element::new("div"),
				related!(Attributes[(
					Attribute::new("class"),
					Value::str("hero light-scheme")
				)]),
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

	#[test]
	fn iter_preserves_class_attributes() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Element::new("button"),
				related!(Attributes[(
					Attribute::new("class"),
					Value::str("primary filled")
				)]),
			))
			.id();

		world.with_state::<ElementQuery, _>(|query| {
			let element = query.iter().find(|el| el.entity == entity).unwrap();
			element.contains_class("primary").xpect_true();
			element.contains_class("filled").xpect_true();
		});
	}
}

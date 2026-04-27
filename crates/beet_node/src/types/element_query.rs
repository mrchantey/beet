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
	pub fn iter(&self) -> impl Iterator<Item = ElementView<'_>> {
		self.elements.iter().map(|(entity, element, attrs, state)| {
			let attributes = attrs
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
				.unwrap_or_default();
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
				let attributes = attrs
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
					.unwrap_or_default();
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
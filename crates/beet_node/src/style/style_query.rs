use crate::style::*;
use beet_core::prelude::*;

#[derive(SystemParam)]
pub struct StyleQuery<'w, 's, T: 'static + Send + Sync> {
	token_store: Res<'w, TokenStore<T>>,
	token_maps: Query<'w, 's, &'static TokenMap<T>>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	props: Query<'w, 's, &'static PropertyMap<T>>,
	resolved_property_maps:
		Query<'w, 's, (Entity, &'static mut ResolvedPropertyMap<T>)>,
}




impl<'w, 's, T: 'static + Send + Sync + PartialEq + Clone>
	StyleQuery<'w, 's, T>
{
	pub fn collect_properties(
		&self,
		entity: Entity,
	) -> Result<HashMap<Property<T>, Token<T>>> {
		let mut map = HashMap::<Property<T>, Token<T>>::new();
		let mut ancestors = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.collect::<Vec<_>>();
		// iter from root for correct override
		ancestors.reverse();
		for ancestor in ancestors.iter() {
			if let Ok(props) = self.props.get(*ancestor) {
				for (key, value) in props.iter() {
					if key.should_inherit() || *ancestor == entity {
						map.insert(key.clone(), value.clone());
					}
				}
			}
		}
		Ok(map)
	}

	pub fn apply_token_maps(
		&self,
		entity: Entity,
		properties: &mut HashMap<Property<T>, Token<T>>,
	) {
		let mut ancestors = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.filter_map(|entity| self.token_maps.get(entity).ok())
			.collect::<Vec<_>>();
		// iter from root for correct override
		ancestors.reverse();
		for ancestor in ancestors.iter() {
			for token in properties.values_mut() {
				if let Some(mapped) = ancestor.get(token) {
					*token = mapped.clone();
				}
			}
		}
	}

	pub fn collect_resolved_properties(
		&self,
		entity: Entity,
	) -> Result<HashMap<Property<T>, T>> {
		let mut properties = self.collect_properties(entity)?;
		self.apply_token_maps(entity, &mut properties);
		let mut map = HashMap::new();
		for (key, token) in properties.into_iter() {
			if let Some(value) = self.token_store.get(&token) {
				map.insert(key, value.clone());
			} else {
				bevybail!("Token not found in store: {:?}", token.to_css_key());
			}
		}
		Ok(map)
	}

	pub fn apply_resolved_properties(&mut self) -> Result {
		for (entity, map) in self
			.resolved_property_maps
			.iter()
			.map(|(entity, _)| entity)
			.map(|entity| (entity, self.collect_resolved_properties(entity)))
			.collect::<Vec<_>>()
		{
			let map = map?;
			self.resolved_property_maps
				.get_mut(entity)?
				.1
				.set_if_neq(ResolvedPropertyMap::new(map));
		}
		Ok(())
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	fn default_styles() -> impl Bundle {
		OnSpawn::merge(
			PropertyMap::<Color>::default()
				.with(props::BACKGROUND_COLOR, colors::PRIMARY)
				.with(props::FOREGROUND_COLOR, colors::ON_PRIMARY),
		)
	}

	fn override_styles() -> impl Bundle {
		OnSpawn::merge(
			PropertyMap::<Color>::default()
				.with(props::FOREGROUND_COLOR, colors::SECONDARY),
		)
	}

	#[test]
	fn test_name() {
		let mut world = World::new();
		world.insert_resource(themes::from_color(palettes::basic::RED));
		let entity = world
			.spawn((schemes::light(), children![
				schemes::dark(),
				default_styles(),
				override_styles(),
				ResolvedPropertyMap::<Color>::default(),
			]))
			.id();
		world
			.with_state::<StyleQuery<Color>, _>(|mut query| {
				query.apply_resolved_properties()
			})
			.unwrap();

		let _map = world
			.entity(entity)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();
		// TODO manually calculate correct token and perform assertions
	}
}

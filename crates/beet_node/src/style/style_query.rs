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
	/// Collects all applicable properties for `entity` by traversing ancestors
	/// root-first, applying inheritance rules at each level.
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

	/// Remaps property tokens using ancestor [`TokenMap`] components.
	///
	/// All ancestor maps are merged first (nearest ancestor wins), then
	/// applied in a single pass. This ensures a child's scheme fully
	/// overrides a parent's scheme for the same semantic tokens.
	pub fn apply_token_maps(
		&self,
		entity: Entity,
		properties: &mut HashMap<Property<T>, Token<T>>,
	) {
		// Collect ancestor token maps from entity outward, then reverse to root-first.
		let mut ancestors: Vec<_> = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.filter_map(|e| self.token_maps.get(e).ok())
			.collect();
		ancestors.reverse(); // root to entity

		// Merge all maps: child entries overwrite parent entries.
		let mut merged: HashMap<Token<T>, Token<T>> = HashMap::new();
		for ancestor in &ancestors {
			for (from, to) in ancestor.iter() {
				merged.insert(from.clone(), to.clone());
			}
		}

		// Apply the merged map once to all property tokens.
		for token in properties.values_mut() {
			if let Some(mapped) = merged.get(token) {
				*token = mapped.clone();
			}
		}
	}

	/// Resolves all properties for `entity` to their concrete values.
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

	/// Resolves and writes [`ResolvedPropertyMap`] for every matching entity.
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

	// ── helpers ──────────────────────────────────────────────────────────────

	fn red_world() -> World {
		let mut world = World::new();
		world.insert_resource(themes::from_color(palettes::basic::RED));
		world
	}

	fn run_style_query(world: &mut World) {
		world
			.with_state::<StyleQuery<Color>, _>(|mut query| {
				query.apply_resolved_properties()
			})
			.unwrap();
	}

	/// Resolves a semantic token through `scheme` to its concrete color.
	fn scheme_color(
		world: &World,
		scheme: &TokenMap<Color>,
		semantic: Token<Color>,
	) -> Color {
		let store = world.resource::<TokenStore<Color>>();
		let tone = scheme.get(&semantic).expect("semantic token not in scheme");
		*store.get(tone).expect("tone not in store")
	}

	fn bg_prop() -> Property<Color> { props::BACKGROUND_COLOR.into() }
	fn fg_prop() -> Property<Color> { props::FOREGROUND_COLOR.into() }

	fn first_child(world: &World, parent: Entity) -> Entity {
		world.entity(parent).get::<Children>().unwrap()[0]
	}

	// ── tests ─────────────────────────────────────────────────────────────────

	/// A single entity with a scheme and [`PropertyMap`] resolves to the correct colors.
	#[test]
	fn resolves_basic_properties() {
		let mut world = red_world();
		let entity = world
			.spawn((
				schemes::light(),
				PropertyMap::<Color>::default()
					.with(props::BACKGROUND_COLOR, colors::PRIMARY)
					.with(props::FOREGROUND_COLOR, colors::ON_PRIMARY),
				ResolvedPropertyMap::<Color>::default(),
			))
			.id();

		run_style_query(&mut world);

		let expected_bg =
			scheme_color(&world, &schemes::light(), colors::PRIMARY);
		let expected_fg =
			scheme_color(&world, &schemes::light(), colors::ON_PRIMARY);

		let resolved = world
			.entity(entity)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();
		resolved.get(&bg_prop()).unwrap().xpect_eq(expected_bg);
		resolved.get(&fg_prop()).unwrap().xpect_eq(expected_fg);
	}

	/// A child's dark scheme fully overrides the parent's light scheme
	/// for the same semantic tokens, verifying the merge-then-apply approach.
	#[test]
	fn child_scheme_overrides_parent() {
		let mut world = red_world();
		let root = world
			.spawn((schemes::light(), children![(
				schemes::dark(),
				PropertyMap::<Color>::default()
					.with(props::BACKGROUND_COLOR, colors::PRIMARY),
				ResolvedPropertyMap::<Color>::default(),
			)]))
			.id();

		run_style_query(&mut world);

		let child = first_child(&world, root);
		// Expected: dark-scheme tone (PRIMARY_80), not light-scheme (PRIMARY_40).
		let expected = scheme_color(&world, &schemes::dark(), colors::PRIMARY);

		let resolved = world
			.entity(child)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();
		resolved.get(&bg_prop()).unwrap().xpect_eq(expected);
	}

	/// Inheritable properties (`FOREGROUND_COLOR`, `inherit = true`) propagate
	/// from a parent entity to a child that has no [`PropertyMap`] of its own.
	#[test]
	fn inherited_property_propagates() {
		let mut world = red_world();
		let root = world
			.spawn((
				schemes::light(),
				PropertyMap::<Color>::default()
					.with(props::FOREGROUND_COLOR, colors::ON_PRIMARY),
				children![ResolvedPropertyMap::<Color>::default()],
			))
			.id();

		run_style_query(&mut world);

		let child = first_child(&world, root);
		let expected =
			scheme_color(&world, &schemes::light(), colors::ON_PRIMARY);

		let resolved = world
			.entity(child)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();
		resolved.get(&fg_prop()).unwrap().xpect_eq(expected);
	}

	/// Non-inheritable properties (`BACKGROUND_COLOR`, `inherit = false`) must
	/// not appear in a child entity's resolved map when set only on the parent.
	#[test]
	fn non_inherited_property_does_not_propagate() {
		let mut world = red_world();
		let root = world
			.spawn((
				schemes::light(),
				PropertyMap::<Color>::default()
					.with(props::BACKGROUND_COLOR, colors::PRIMARY),
				children![ResolvedPropertyMap::<Color>::default()],
			))
			.id();

		run_style_query(&mut world);

		let child = first_child(&world, root);
		let resolved = world
			.entity(child)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();
		resolved.get(&bg_prop()).xpect_none();
	}
}

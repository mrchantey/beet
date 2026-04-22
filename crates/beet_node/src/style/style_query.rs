use crate::style::*;
use beet_core::prelude::*;

#[derive(SystemParam)]
pub struct StyleQuery<'w, 's, T: 'static + Send + Sync> {
	// token stores
	global_token_store: Res<'w, TokenStore<T>>,
	token_stores: Query<'w, 's, (Entity, &'static TokenStore<T>)>,
	// token maps
	global_token_map: Option<Res<'w, TokenMap>>,
	token_maps: Query<'w, 's, (Entity, &'static TokenMap)>,
	// props
	props: Query<'w, 's, (Entity, &'static PropertyMap)>,
	resolved_property_maps:
		Query<'w, 's, (Entity, &'static mut ResolvedPropertyMap<T>)>,
	// utils
	ancestors: Query<'w, 's, &'static ChildOf>,
}

impl<'w, 's, T: 'static + Send + Sync + PartialEq + Clone>
	StyleQuery<'w, 's, T>
{
	/// Collects all applicable properties for `entity` by traversing ancestors
	/// root-first, applying inheritance rules at each level.
	pub fn collect_properties(
		&self,
		entity: Entity,
	) -> Result<HashMap<Property, Token>> {
		let mut map = HashMap::<Property, Token>::new();
		let mut ancestors = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.collect::<Vec<_>>();
		// iter from root for correct override
		ancestors.reverse();
		for ancestor in ancestors.iter() {
			if let Ok((_, props)) = self.props.get(*ancestor) {
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
		properties: &mut HashMap<Property, Token>,
	) {
		// Collect ancestor token maps from entity outward, then reverse to root-first.
		let mut token_maps: Vec<_> = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.filter_map(|entity| {
				self.token_maps.get(entity).ok().map(|(_, map)| map)
			})
			.collect();
		if let Some(global) = self.global_token_map.as_ref() {
			token_maps.push(global);
		}
		// global to root to entity
		token_maps.reverse();

		// Merge all maps: child entries overwrite parent entries.
		let mut merged: HashMap<Token, Token> = HashMap::new();
		for map in &token_maps {
			for (from, to) in map.iter() {
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
	) -> Result<HashMap<Property, TokenValue<T>>> {
		let mut properties = self.collect_properties(entity)?;
		self.apply_token_maps(entity, &mut properties);
		let mut map = HashMap::new();
		for (key, token) in properties.into_iter() {
			if let Some(value) = self.resolve_token_value(entity, &token) {
				map.insert(key, value);
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

	/// Validates token and property type tags across all stores and maps.
	pub fn validate_tokens(&self) -> Result<(), ValidateTokensError>
	where
		T: TypeTag,
	{
		let mut token_store = Vec::new();
		let mut token_map = Vec::new();
		let mut property = Vec::new();

		for (entity, store) in self.token_stores.iter() {
			for (token, value) in store.iter() {
				if token.type_tag().as_str() != value.type_tag().as_str() {
					token_store.push((
						Some(entity),
						token.clone(),
						token.clone(),
					));
				}
			}
		}

		for (token, value) in self.global_token_store.iter() {
			if token.type_tag().as_str() != value.type_tag().as_str() {
				token_store.push((None, token.clone(), token.clone()));
			}
		}

		if let Some(global_map) = self.global_token_map.as_ref() {
			for (from, to) in global_map.iter() {
				if from.type_tag() != to.type_tag() {
					token_map.push((None, from.clone(), to.clone()));
				}
			}
		}

		for (entity, map) in self.token_maps.iter() {
			for (from, to) in map.iter() {
				if from.type_tag() != to.type_tag() {
					token_map.push((Some(entity), from.clone(), to.clone()));
				}
			}
		}

		for (entity, props) in self.props.iter() {
			for (prop, token) in props.iter() {
				if prop.def().type_tag() != token.type_tag() {
					property.push((entity, prop.def().clone(), token.clone()));
				}
			}
		}

		if token_store.is_empty() && token_map.is_empty() && property.is_empty()
		{
			Ok(())
		} else {
			Err(ValidateTokensError::TypeMismatch {
				token_store,
				token_map,
				property,
			})
		}
	}

	fn resolve_token_value(
		&self,
		entity: Entity,
		token: &Token,
	) -> Option<TokenValue<T>> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| {
				self.token_stores
					.get(ancestor)
					.ok()
					.and_then(|(_, store)| store.get(token).cloned())
			})
			.or_else(|| self.global_token_store.get(token).cloned())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ValidateTokensError {
	#[error("style token type mismatch")]
	TypeMismatch {
		/// Entity is `None` when the mismatch came from a resource.
		token_store: Vec<(Option<Entity>, Token, Token)>,
		/// Entity is `None` when the mismatch came from a resource.
		token_map: Vec<(Option<Entity>, Token, Token)>,
		property: Vec<(Entity, PropertyDef, Token)>,
	},
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
				query.validate_tokens()?;
				query.apply_resolved_properties()
			})
			.unwrap();
	}

	/// Resolves a semantic token through `scheme` to its concrete color.
	fn scheme_color(
		world: &World,
		scheme: &TokenMap,
		semantic: Token,
	) -> Color {
		let store = world.resource::<TokenStore<Color>>();
		let tone = scheme.get(&semantic).unwrap();
		match store.get(tone).unwrap() {
			TokenValue::Color(value) => *value,
			other => panic!("expected color style value, found {other:?}"),
		}
	}

	fn bg_prop() -> Property { props::BACKGROUND_COLOR.into() }
	fn fg_prop() -> Property { props::FOREGROUND_COLOR.into() }

	fn first_child(world: &World, parent: Entity) -> Entity {
		world.entity(parent).get::<Children>().unwrap()[0]
	}

	// ── tests ─────────────────────────────────────────────────────────────────

	#[test]
	fn resolves_basic_properties() {
		let mut world = red_world();
		let entity = world
			.spawn((
				schemes::light(),
				PropertyMap::default()
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

		match resolved.get(&bg_prop()).unwrap() {
			TokenValue::Color(value) => {
				value.xpect_eq(expected_bg);
			}
			other => panic!("expected color style value, found {other:?}"),
		}
		match resolved.get(&fg_prop()).unwrap() {
			TokenValue::Color(value) => {
				value.xpect_eq(expected_fg);
			}
			other => panic!("expected color style value, found {other:?}"),
		}
	}

	#[test]
	fn child_scheme_overrides_parent() {
		let mut world = red_world();
		let root = world
			.spawn((schemes::light(), children![(
				schemes::dark(),
				PropertyMap::default()
					.with(props::BACKGROUND_COLOR, colors::PRIMARY),
				ResolvedPropertyMap::<Color>::default(),
			)]))
			.id();

		run_style_query(&mut world);

		let child = first_child(&world, root);
		let expected = scheme_color(&world, &schemes::dark(), colors::PRIMARY);

		let resolved = world
			.entity(child)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();

		match resolved.get(&bg_prop()).unwrap() {
			TokenValue::Color(value) => {
				value.xpect_eq(expected);
			}
			other => panic!("expected color style value, found {other:?}"),
		}
	}

	#[test]
	fn inherited_property_propagates() {
		let mut world = red_world();
		let root = world
			.spawn((
				schemes::light(),
				PropertyMap::default()
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

		match resolved.get(&fg_prop()).unwrap() {
			TokenValue::Color(value) => {
				value.xpect_eq(expected);
			}
			other => panic!("expected color style value, found {other:?}"),
		}
	}

	#[test]
	fn non_inherited_property_does_not_propagate() {
		let mut world = red_world();
		let root = world
			.spawn((
				schemes::light(),
				PropertyMap::default()
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

	#[test]
	fn token_store_component_overrides_resource() {
		let mut world = red_world();
		// The light scheme remaps colors::PRIMARY -> tones::PRIMARY_40,
		// so the entity store must override the tone, not the semantic color.
		let root = world
			.spawn((
				TokenStore::<Color>::new()
					.with(tones::PRIMARY_40, Color::WHITE),
				schemes::light(),
				PropertyMap::default()
					.with(props::BACKGROUND_COLOR, colors::PRIMARY),
				ResolvedPropertyMap::<Color>::default(),
			))
			.id();

		run_style_query(&mut world);

		let resolved = world
			.entity(root)
			.get::<ResolvedPropertyMap<Color>>()
			.unwrap();

		match resolved.get(&bg_prop()).unwrap() {
			TokenValue::Color(value) => {
				value.xpect_eq(Color::WHITE);
			}
			other => panic!("expected color style value, found {other:?}"),
		}
	}

	#[test]
	fn validate_tokens_accepts_matching_tags() {
		let mut world = red_world();
		world.spawn((
			schemes::light(),
			PropertyMap::default()
				.with(props::BACKGROUND_COLOR, colors::PRIMARY),
		));

		world
			.with_state::<StyleQuery<Color>, _>(|query| query.validate_tokens())
			.unwrap();
	}

	#[test]
	fn validate_tokens_rejects_property_type_mismatch() {
		let mut world = red_world();
		// Map a color property to a unit token — clear type mismatch.
		let unit_token = Token::new_static::<Unit>("space-sm");
		let entity = world
			.spawn(
				PropertyMap::default()
					.with(props::BACKGROUND_COLOR, unit_token.clone()),
			)
			.id();

		let err = world
			.with_state::<StyleQuery<Color>, _>(|query| query.validate_tokens())
			.unwrap_err();

		match err {
			ValidateTokensError::TypeMismatch { property, .. } => {
				property.len().xpect_eq(1);
				property[0].0.xpect_eq(entity);
				property[0].1.xpect_eq(props::BACKGROUND_COLOR.clone());
				property[0].2.xpect_eq(unit_token);
			}
		}
	}

	#[test]
	fn validate_tokens_rejects_token_map_type_mismatch() {
		let mut world = red_world();
		let entity =
			world
				.spawn(TokenMap::default().with(
					colors::PRIMARY,
					Token::new_static::<Unit>("space-sm"),
				))
				.id();

		let err = world
			.with_state::<StyleQuery<Color>, _>(|query| query.validate_tokens())
			.unwrap_err();

		match err {
			ValidateTokensError::TypeMismatch { token_map, .. } => {
				token_map.len().xpect_eq(1);
				token_map[0].0.xpect_eq(Some(entity));
				token_map[0].1.xpect_eq(colors::PRIMARY);
				token_map[0]
					.2
					.xpect_eq(Token::new_static::<Unit>("space-sm"));
			}
		}
	}

	#[test]
	fn validate_tokens_rejects_token_store_type_mismatch() {
		let mut world = red_world();
		let entity = world
			.spawn(
				TokenStore::<Color>::new()
					.with(colors::PRIMARY, TokenValue::Unit(Unit::Px(4.0))),
			)
			.id();

		let err = world
			.with_state::<StyleQuery<Color>, _>(|query| query.validate_tokens())
			.unwrap_err();

		match err {
			ValidateTokensError::TypeMismatch { token_store, .. } => {
				token_store.len().xpect_eq(1);
				token_store[0].0.xpect_eq(Some(entity));
				token_store[0].1.xpect_eq(colors::PRIMARY);
				token_store[0].2.xpect_eq(colors::PRIMARY);
			}
		}
	}
}

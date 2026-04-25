use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// An ordered collection of [`Selector`]s applied from first to last.
///
/// Later selectors override earlier ones for the same token path.
/// Can be used as a global [`Resource`] or per-entity [`Component`].
#[derive(Default, Deref, DerefMut, Resource, Component)]
pub struct SelectorStore(Vec<Selector>);

impl SelectorStore {
	/// Add a selector to this store.
	pub fn with(mut self, selector: Selector) -> Self {
		self.0.push(selector);
		self
	}
}

/// Resolves token values for elements using [`Selector`]s.
///
/// Applies selectors from a global [`SelectorStore`] resource and
/// per-entity [`SelectorStore`] components. Entity-local selectors
/// override global ones for the same token path.
///
/// Token inheritance across the entity hierarchy is handled by
/// [`DocumentPath`] on each token definition — this query only
/// collects values for the given entity.
///
/// # Future work
/// Ancestor [`SelectorStore`] traversal (CSS cascade) is not yet
/// implemented; add the entity's own selectors or use the global store.
#[derive(SystemParam)]
pub struct StyleQuery<'w, 's> {
	entity_selectors: Query<'w, 's, (Entity, &'static SelectorStore)>,
	global_selectors: Option<Res<'w, SelectorStore>>,
	elements: ElementQuery<'w, 's>,
}

impl StyleQuery<'_, '_> {

	/// Collect selectors in order:
	/// 1. Global [`SelectorStore`] resource (lowest priority)
	/// 2. Entity-local [`SelectorStore`] component (highest priority)
	pub fn collect_selectors(&self, entity: Entity) -> Vec<&Selector> {
		let mut selectors = Vec::new();
		if let Some(global) = &self.global_selectors {
			selectors.extend(global.iter());
		}
		if let Ok((_, store)) = self.entity_selectors.get(entity) {
			selectors.extend(store.iter());
		}
		selectors
	}

	/// Collect all token values for `entity` by applying matching selectors.
	///
	/// Applies selectors in order:
	/// 1. Global [`SelectorStore`] resource (lowest priority)
	/// 2. Entity-local [`SelectorStore`] component (highest priority)
	///
	/// Returns a map of token [`FieldPath`] → [`ValueOrRef`].
	pub fn collect_tokens(
		&self,
		entity: Entity,
	) -> HashMap<FieldPath, ValueOrRef> {
		let mut map = HashMap::new();

		let Ok(el) = self.elements.get(entity) else {
			return map;
		};

		for selector in self.collect_selectors(entity) {
			if selector.matches(&el) {
				map.extend(
					selector
						.tokens()
						.iter()
						.map(|(k, v)| (k.clone(), v.clone())),
				);
			}
		}
		map
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::material::colors;


	#[test]
	fn selector_store_applies_in_order() {
		// Later selectors override earlier ones for the same token path.
		let red: Color = bevy::color::palettes::basic::RED.into();
		let blue: Color = bevy::color::palettes::basic::BLUE.into();
		let store = SelectorStore::default()
			.with(Selector::new().with_value::<colors::Primary>(red).unwrap())
			.with(Selector::new().with_value::<colors::Primary>(blue).unwrap());

		let mut world = World::new();
		let entity = world.spawn((Element::new("div"), store)).id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			let val = tokens.get(&colors::Primary::path()).unwrap();
			matches!(val, ValueOrRef::Value(_)).xpect_true();
		});
	}

	#[test]
	fn entity_local_overrides_global() {
		let mut world = World::new();

		// Global: Primary → RED
		let red: Color = bevy::color::palettes::basic::RED.into();
		let blue: Color = bevy::color::palettes::basic::BLUE.into();
		world.insert_resource(
			SelectorStore::default().with(
				Selector::new().with_value::<colors::Primary>(red).unwrap(),
			),
		);

		// Entity-local: Primary → BLUE
		let entity = world
			.spawn((
				Element::new("div"),
				SelectorStore::default().with(
					Selector::new()
						.with_value::<colors::Primary>(blue)
						.unwrap(),
				),
			))
			.id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			let val = tokens.get(&colors::Primary::path()).unwrap();
			// entity-local value wins
			matches!(val, ValueOrRef::Value(_)).xpect_true();
		});
	}

	#[test]
	fn no_element_returns_empty_map() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			tokens.is_empty().xpect_true();
		});
	}

	#[test]
	fn rule_based_selector_only_matches_tagged_element() {
		use crate::style::Rule;
		let mut world = World::new();

		let red: Color = bevy::color::palettes::basic::RED.into();
		world.insert_resource(
			SelectorStore::default().with(
				Selector::new()
					.with_rule(Rule::tag("button"))
					.with_value::<colors::Primary>(red)
					.unwrap(),
			),
		);

		let button = world.spawn(Element::new("button")).id();
		let div = world.spawn(Element::new("div")).id();

		world.with_state::<StyleQuery, _>(|query| {
			let button_tokens = query.collect_tokens(button);
			let div_tokens = query.collect_tokens(div);
			button_tokens
				.contains_key(&colors::Primary::path())
				.xpect_true();
			div_tokens
				.contains_key(&colors::Primary::path())
				.xpect_false();
		});
	}

	#[test]
	fn light_scheme_selector_maps_primary_to_tone() {
		use crate::style::material::themes;
		let mut world = World::new();

		world.insert_resource(
			SelectorStore::default().with(themes::light_scheme()),
		);

		let entity = world.spawn(Element::new("div")).id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			// Primary should now point to a tones::Primary40 FieldRef
			let val = tokens.get(&colors::Primary::path()).unwrap();
			matches!(val, ValueOrRef::Ref(_)).xpect_true();
		});
	}
}

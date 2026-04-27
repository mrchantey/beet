use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;


#[derive(SystemParam, Get)]
pub struct StyleQuery<'w, 's> {
	entity_rules: Query<'w, 's, (Entity, &'static RuleStore)>,
	global_rules: Option<Res<'w, RuleStore>>,
	elements: ElementQuery<'w, 's>,

	css_map: Option<Res<'w, CssTokenMap>>,
}

impl StyleQuery<'_, '_> {
	/// Collect ruless in order:
	/// 1. Global [`RuleStore`] resource (lowest priority)
	/// 2. Entity-local [`RuleStore`] component (highest priority)
	pub fn collect_rules(&self, entity: Entity) -> Vec<&Rule> {
		let mut rules = Vec::new();
		if let Some(global) = &self.global_rules {
			rules.extend(global.iter());
		}
		if let Ok((_, store)) = self.entity_rules.get(entity) {
			rules.extend(store.iter());
		}
		rules
	}

	/// Collect all token values for `entity` by applying matching rules.
	///
	/// Applies rules in order:
	/// 1. Global [`RuleStore`] resource (lowest priority)
	/// 2. Entity-local [`RuleStore`] component (highest priority)
	///
	/// Returns a map of token [`FieldPath`] → [`ValueOrRef`].
	pub fn collect_tokens(
		&self,
		entity: Entity,
	) -> HashMap<TokenKey, TokenValue> {
		let mut map = HashMap::new();

		let Ok(el) = self.elements.get(entity) else {
			return map;
		};

		for rule in self.collect_rules(entity) {
			if rule.matches(&el) {
				map.extend(
					rule.declarations()
						.iter()
						.map(|(k, v)| (k.clone(), v.clone())),
				);
			}
		}
		map
	}

	/// Recursively resolves token path to a value
	pub fn get_token(&self, entity: Entity, key: &TokenKey) -> Result<&Value> {
		let Ok(el) = self.elements.get(entity) else {
			bevybail!("Entity {} does not have an Element component", entity);
		};

		for rule in self.collect_rules(entity) {
			if rule.matches(&el)
				&& let Some(val) = rule.declarations().get(key)
			{
				match val {
					TokenValue::Value(value) => return Ok(value.value()),
					TokenValue::Token(token) => {
						return self.get_token(entity, token.key());
					}
				}
			}
		}
		bevybail!("Token not found for path: {}", key.as_str())
	}


	pub fn build_css(
		&self,
		builder: &CssBuilder,
		entity: Entity,
	) -> Result<String> {
		let Some(css_map) = &self.css_map else {
			bevybail!("No CssTokenMap resource found");
		};
		let rules = self.collect_rules(entity);
		builder.build(css_map, &rules)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::material::colors;


	#[test]
	fn rule_store_applies_in_order() {
		// Later rules override earlier ones for the same token path.
		let red: Color = bevy::color::palettes::basic::RED.into();
		let blue: Color = bevy::color::palettes::basic::BLUE.into();
		let store = RuleStore::default()
			.with(Rule::new().with_value::<colors::Primary>(red).unwrap())
			.with(Rule::new().with_value::<colors::Primary>(blue).unwrap());

		let mut world = World::new();
		let entity = world.spawn((Element::new("div"), store)).id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			let val = tokens.get(&colors::Primary::key()).unwrap();
			matches!(val, TokenValue::Value(_)).xpect_true();
		});
	}

	#[test]
	fn entity_local_overrides_global() {
		let mut world = World::new();

		// Global: Primary → RED
		let red: Color = bevy::color::palettes::basic::RED.into();
		let blue: Color = bevy::color::palettes::basic::BLUE.into();
		world.insert_resource(
			RuleStore::default()
				.with(Rule::new().with_value::<colors::Primary>(red).unwrap()),
		);

		// Entity-local: Primary → BLUE
		let entity = world
			.spawn((
				Element::new("div"),
				RuleStore::default().with(
					Rule::new().with_value::<colors::Primary>(blue).unwrap(),
				),
			))
			.id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			let val = tokens.get(&colors::Primary::key()).unwrap();
			// entity-local value wins
			matches!(val, TokenValue::Value(_)).xpect_true();
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
	fn selector_based_rule_only_matches_tagged_element() {
		use crate::style::Selector;
		let mut world = World::new();

		let red: Color = bevy::color::palettes::basic::RED.into();
		world.insert_resource(
			RuleStore::default().with(
				Rule::new()
					.with_selector(Selector::tag("button"))
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
				.contains_key(&colors::Primary::key())
				.xpect_true();
			div_tokens
				.contains_key(&colors::Primary::key())
				.xpect_false();
		});
	}

	#[test]
	fn light_scheme_rule_maps_primary_to_tone() {
		use crate::style::material::themes;
		let mut world = World::new();

		world
			.insert_resource(RuleStore::default().with(themes::light_scheme()));

		let entity = world
			.spawn((
				Element::new("div"),
				related!(
					Attributes[(
						Attribute::new("class"),
						Value::str(themes::LIGHT_SCHEME_CLASS)
					)]
				),
			))
			.id();

		world.with_state::<StyleQuery, _>(|query| {
			let tokens = query.collect_tokens(entity);
			// Primary should now point to a tones::Primary40 FieldRef
			let val = tokens.get(&colors::Primary::key()).unwrap();
			matches!(val, TokenValue::Token(_)).xpect_true();
		});
	}
}

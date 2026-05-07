use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;


#[derive(SystemParam, Get)]
pub struct StyleQuery<'w, 's> {
	entity_tokens: Query<'w, 's, (Entity, &'static TokenStore)>,
	global_tokens: Option<Res<'w, TokenStore>>,
	elements: ElementQuery<'w, 's>,

	css_map: Option<Res<'w, CssTokenMap>>,
}

impl StyleQuery<'_, '_> {
	/// Collect ruless in order:
	/// 1. Global [`RuleStore`] resource (lowest priority)
	/// 2. Entity-local [`RuleStore`] component (highest priority)
	pub fn collect_token_store(
		&self,
		entity: Entity,
	) -> Vec<(&TokenKey, &TokenValue)> {
		let mut rules = Vec::new();
		if let Some(global) = &self.global_tokens {
			rules.extend(global.iter());
		}
		if let Ok((_, store)) = self.entity_tokens.get(entity) {
			rules.extend(store.iter());
		}

		rules
	}

	pub fn build_css(
		&self,
		builder: &CssBuilder,
		entity: Entity,
	) -> Result<String> {
		let Some(css_map) = &self.css_map else {
			bevybail!("No CssTokenMap resource found");
		};
		let tokens = self.collect_token_store(entity);
		builder.build(css_map, &tokens)
	}
}

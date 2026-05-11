use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// System parameter for building CSS from the global [`RuleSet`] resource.
#[derive(SystemParam, Get)]
pub struct StyleQuery<'w, 's> {
	rule_set: Option<Res<'w, RuleSet>>,
	elements: ElementQuery<'w, 's>,
	css_map: Option<Res<'w, CssTokenMap>>,
}

impl StyleQuery<'_, '_> {
	/// Builds a CSS string from the global [`RuleSet`] resource.
	///
	/// The `_entity` parameter is retained for API compatibility.
	pub fn build_css(
		&self,
		builder: &CssBuilder,
		_entity: Entity,
	) -> Result<String> {
		let Some(css_map) = &self.css_map else {
			bevybail!("No CssTokenMap resource found");
		};
		let Some(rule_set) = &self.rule_set else {
			bevybail!("No RuleSet resource found");
		};
		let rules: Vec<Rule> = rule_set.rules().cloned().collect();
		builder.build(css_map, &rules)
	}
}

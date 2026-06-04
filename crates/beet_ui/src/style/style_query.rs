use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// System parameter for building CSS from the global [`RuleSet`] resource.
#[derive(SystemParam, Get)]
pub struct StyleQuery<'w, 's> {
	rule_set: Res<'w, RuleSet>,
	css_map: Res<'w, CssTokenMap>,
	elements: ElementQuery<'w, 's>,
}

impl StyleQuery<'_, '_> {
	/// Builds a CSS string from the global [`RuleSet`] resource.
	///
	/// The `_entity` parameter is retained for API compatibility.
	pub fn build_css(&self, builder: &CssBuilder) -> Result<String> {
		builder.build(&self.css_map, &self.rule_set)
	}
}

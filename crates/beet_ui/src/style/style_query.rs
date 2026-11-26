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
	pub fn build_css(&self, builder: &CssBuilder) -> Result<String> {
		builder.build(&self.css_map, &self.rule_set)
	}

	/// The name each variable takes in the sheet `builder` builds from the
	/// global [`RuleSet`], keyed by its full name.
	pub fn variable_names(
		&self,
		builder: &CssBuilder,
	) -> Result<HashMap<CssVariable, CssVariable>> {
		builder.variable_names(&self.css_map, &self.rule_set)
	}

	/// The CSS of only the rules selecting one of `classes`: the sheet a
	/// fragment rendered apart from its page carries, the page's own sheet
	/// holding the `:root` variables and every other rule. Variable names
	/// format as the whole sheet's do ([`CssBuilder::build_where`]).
	pub fn build_css_selecting(
		&self,
		builder: &CssBuilder,
		classes: &HashSet<SmolStr>,
	) -> Result<String> {
		builder.build_where(&self.css_map, &self.rule_set, |rule| {
			rule.selector().any_class(|class| classes.contains(class))
		})
	}
}

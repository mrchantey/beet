use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Resource)]
pub struct RuleSet {
	rules: Vec<Rule>,
}

/// By default, the rule set is initialized with a single root rule
impl Default for RuleSet {
	fn default() -> Self {
		Self {
			rules: vec![Rule::default()],
		}
	}
}


impl RuleSet {
	pub fn default_rule(&self) -> &Rule {
		self.rules
			.first()
			.expect("RuleSet should always have at least one rule")
	}
	pub fn default_rule_mut(&mut self) -> &mut Rule {
		self.rules
			.first_mut()
			.expect("RuleSet should always have at least one rule")
	}
}

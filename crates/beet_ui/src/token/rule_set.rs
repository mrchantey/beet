use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::VecDeque;


#[derive(Resource)]
pub struct RuleSet {
	rules: VecDeque<Rule>,
}

/// By default, the rule set is initialized with a single root rule
impl Default for RuleSet {
	fn default() -> Self {
		let mut rules = VecDeque::with_capacity(1);
		rules.push_back(Rule::default());
		Self { rules }
	}
}


impl RuleSet {
	pub fn default_rule(&self) -> &Rule {
		self.rules
			.front()
			.expect("RuleSet should always have at least one rule")
	}
	pub fn default_rule_mut(&mut self) -> &mut Rule {
		self.rules
			.front_mut()
			.expect("RuleSet should always have at least one rule")
	}
}

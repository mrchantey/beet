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
	/// Gets the oldest rule, by default this
	/// is a rule with root
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
	pub fn insert_rule(&mut self, rule: Rule) {
		if let Some(recent) = self.rules.back() {
			if recent.selector  {
				// If the most recent rule is a root, we can replace it with the new rule
				*self.rules.back_mut().unwrap() = rule;
				return;
			}
		}else{			
		self.rules.push_back(rule);
		}
	}
}

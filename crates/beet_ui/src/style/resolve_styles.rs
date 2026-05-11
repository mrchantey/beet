use crate::prelude::*;
use beet_core::prelude::*;





pub fn resolve_styles(
	ruleset_query: RuleSetQuery,
	query: Query<Entity, Changed<Element>>,
	ancestors: Query<&ChildOf>,
) {
	// TODO fine-grained resolution
	// listen for class attribute changes,
	// reparenting etc
	let roots = query
		.iter()
		.map(|entity| ancestors.root_ancestor(entity))
		.collect::<HashSet<_>>();

	for root in roots.iter(){
		
	}	
}

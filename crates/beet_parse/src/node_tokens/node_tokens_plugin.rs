use std::cell::RefCell;

use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

/// System step for creating the entities containing nodes and their tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportNodesStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessNodesStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportNodesStep;

#[derive(Default)]
pub struct NodeTokensPlugin;


impl Plugin for NodeTokensPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				ProcessNodesStep.after(ImportNodesStep),
				ExportNodesStep.after(ProcessNodesStep),
				ExtractDirectivesSet.in_set(ProcessNodesStep),
			),
		)
		.add_plugins((
			tokens_to_rstml_plugin,
			rstml_to_node_tokens_plugin,
			combinator_to_node_tokens_plugin,
			extract_rsx_directives_plugin,
			extract_web_directives_plugin,
			tokenize_bundle_plugin,
		));
	}
}



thread_local! {
	static TOKENS_APP: RefCell<Option<App>> = RefCell::new(None);
}


/// Access a shared `thread_local!` [`App`] used by the [`NodeTokensPlugin`].
pub struct TokensApp;

impl TokensApp {
	// The idea is for every macro instance to not have to spin up a new app,
	// but it breaks rust-analyzer for some reason so is currently disabled.
	pub fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
		let mut app = App::new();
		app.add_plugins(NodeTokensPlugin);
		func(&mut app)
	}
	//	TODO static app breaks rust analyzer?
	pub fn with_broken<O>(func: impl FnOnce(&mut App) -> O) -> O {
		TOKENS_APP.with(|app_cell| {
			// Initialize the app if needed
			let mut app_ref = app_cell.borrow_mut();
			if app_ref.is_none() {
				let mut app = App::new();
				app.add_plugins(NodeTokensPlugin);
				*app_ref = Some(app);
			}

			// Now we can safely unwrap and use the app
			let app = app_ref.as_mut().unwrap();

			let out = func(app);
			assert_empty(app);
			out
		})
	}
}

fn assert_empty(app: &App) {
	if app.world().entities().is_empty() {
		return;
	}
	let mut err = String::from("TokensApp contains entities after use\n");
	for entity in app.world().iter_entities() {
		let entity = entity.id();
		let info = app.world().inspect_entity(entity).unwrap();
		err.push_str(&format!(
			"Entity {}\n{}",
			entity,
			info.map(|i| format!("\t{}", i.name()))
				.collect::<Vec<_>>()
				.join("\n")
		));
	}
	panic!("{err}");
}

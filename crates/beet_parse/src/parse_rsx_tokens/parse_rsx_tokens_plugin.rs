use super::*;
use beet_common::prelude::*;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;
use std::cell::RefCell;

/// System set for the [`ParseRsxTokensPlugin`] to extract directives
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ParseRsxTokensSet;

/// A plugin for parsing raw rstml token streams and combinator strings into
/// rsx trees, then extracting directives.
#[derive(Default)]
pub struct ParseRsxTokensPlugin;


impl Plugin for ParseRsxTokensPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<RstmlConfig>()
			.configure_sets(
				Update,
				ExtractDirectivesSet.in_set(ParseRsxTokensSet),
			)
			.add_systems(
				Update,
				(
					(parse_combinator_tokens, parse_rstml_tokens)
						.in_set(ParseRsxTokensSet)
						.before(ExtractDirectivesSet),
					collapse_combinator_exprs
						.in_set(ParseRsxTokensSet)
						.after(ExtractDirectivesSet),
				),
			)
			.add_plugins((
				extract_rsx_directives_plugin,
				extract_web_directives_plugin,
			));

		// cache the rstml parser to avoid recreating every frame
		let rstml_config = app.world().resource::<RstmlConfig>();
		app.insert_non_send_resource(rstml_config.clone().into_parser());
	}
}



thread_local! {
	static TOKENS_APP: RefCell<Option<App>> = RefCell::new(None);
}


/// Access a shared `thread_local!` [`App`] used by the [`NodeTokensPlugin`].
// The idea is to save every single macro from spinning up an app
// but this needs to be benched.
// TODO it breaks rust-analyzer for some reason so is currently disabled.
pub struct TokensApp;

impl TokensApp {
	pub fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
		let mut app = App::new();
		app.add_plugins(ParseRsxTokensPlugin);
		func(&mut app)
	}
	//	TODO static app breaks rust analyzer?
	pub fn with_broken<O>(func: impl FnOnce(&mut App) -> O) -> O {
		TOKENS_APP.with(|app_cell| {
			// Initialize the app if needed
			let mut app_ref = app_cell.borrow_mut();
			if app_ref.is_none() {
				let mut app = App::new();
				app.add_plugins(ParseRsxTokensPlugin);
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

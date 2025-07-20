use super::*;
#[allow(unused)]
use crate::prelude::*;
use beet_core::prelude::*;
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
		app.init_resource::<HtmlConstants>().add_systems(
			Update,
			(
				// parsing raw tokens
				#[cfg(feature = "rsx")]
				parse_combinator_tokens,
				#[cfg(feature = "rsx")]
				parse_rstml_tokens,
				// extractors
				// lang nodes must run first, hashes raw attributes not extracted directives
				extract_lang_nodes,
				extract_slot_targets,
				try_extract_directive::<SlotChild>,
				try_extract_directive::<ClientLoadDirective>,
				try_extract_directive::<ClientOnlyDirective>,
				try_extract_directive::<HtmlHoistDirective>,
				try_extract_directive::<StyleScope>,
				try_extract_directive::<StyleCascade>,
				// collect combinator exprs last
				#[cfg(feature = "rsx")]
				collapse_combinator_exprs,
				#[cfg(feature = "css")]
				parse_lightning,
			)
				.chain()
				.in_set(ParseRsxTokensSet),
		);

		// cache the rstml parser to avoid recreating every frame
		let constants = app.world().resource::<HtmlConstants>();
		let rstml_parser = create_rstml_parser(constants);
		app.insert_non_send_resource(rstml_parser);
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
